import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import test from "node:test"
import {
  createRadeonManager,
  normalizeRadeonDetection,
  normalizeRadeonResult,
  runRadeonHelper,
} from "../src/radeon.mjs"

const nativeSource = await readFile(
  new URL("../native/radeon-helper/main.cpp", import.meta.url),
  "utf8",
)

function helperResult(overrides = {}) {
  return {
    detected: true,
    gpus: [{ name: "AMD Radeon 테스트 GPU" }],
    goals: [
      {
        key: "verticalSyncOff",
        label: "수직 동기화 항상 끄기",
        supported: true,
        met: true,
        currentValue: "항상 끄기",
      },
      {
        key: "enhancedSyncOff",
        label: "Enhanced Sync 끄기",
        supported: false,
        met: false,
        currentValue: "지원 안 함",
      },
      {
        key: "antiLagOn",
        label: "Radeon Anti-Lag 켜기",
        supported: true,
        met: true,
        currentValue: "켜기",
      },
    ],
    allMet: true,
    tuningAccessed: false,
    reason: null,
    ...overrides,
  }
}

test("Radeon 시작 상태는 helper 없이 WMI 장치와 적용 전 목표만 표시한다", async () => {
  let helperCalls = 0
  const manager = createRadeonManager({
    runHelper: async () => {
      helperCalls += 1
      return helperResult()
    },
    detectDevices: async () => ({
      gpus: [{ name: "AMD Radeon RX 6700 XT" }],
    }),
  })

  const result = await manager.check()

  assert.equal(result.vendor, "amd")
  assert.equal(result.goalsList.length, 4)
  assert.equal(result.goalsList[0].pending, true)
  assert.equal(result.tuningProtected, true)
  assert.equal(helperCalls, 0)
})

test("Radeon WMI 장치 이름 누락은 안전한 이름으로 정규화한다", () => {
  const result = normalizeRadeonDetection({ gpus: [{ name: null }] })

  assert.equal(result.detected, true)
  assert.equal(result.gpus[0].name, "AMD Radeon GPU")
})

test("지원하지 않는 Radeon 목표는 완료 판정에서 제외한다", () => {
  const result = normalizeRadeonResult(
    helperResult(),
    { available: true, enabled: false },
  )

  assert.equal(result.vendor, "amd")
  assert.equal(result.allMet, true)
  assert.equal(result.goalsList[1].supported, false)
})

test("마비노기 내부 수직 동기화가 켜져 있으면 목표가 미완료다", () => {
  const result = normalizeRadeonResult(
    helperResult(),
    { available: true, enabled: true },
  )

  assert.equal(result.allMet, false)
  assert.equal(result.goalsList[0].met, false)
  assert.match(result.goalsList[0].currentValue, /게임 설정 켜기/)
})

test("Radeon 적용은 전역 helper 적용 후 게임 수직 동기화를 끄고 검증한다", async () => {
  const calls = []
  let gameEnabled = true
  const manager = createRadeonManager({
    runHelper: async apply => {
      calls.push(["helper", apply])
      return helperResult()
    },
    queryGameVerticalSync: async () => ({
      available: true,
      enabled: gameEnabled,
    }),
    setGameVerticalSync: async enabled => {
      calls.push(["game", enabled])
      gameEnabled = enabled
    },
  })

  const result = await manager.apply()

  assert.equal(result.allMet, true)
  assert.deepEqual(calls, [["helper", true], ["game", false]])
})

test("Radeon 적용 후 남은 목표가 있으면 실패한다", async () => {
  const manager = createRadeonManager({
    runHelper: async () => helperResult({
      goals: [
        {
          key: "antiLagOn",
          label: "Radeon Anti-Lag 켜기",
          supported: true,
          met: false,
          currentValue: "끄기",
        },
      ],
    }),
    queryGameVerticalSync: async () => ({ available: true, enabled: false }),
    setGameVerticalSync: async () => {},
  })

  await assert.rejects(() => manager.apply(), /Radeon Anti-Lag 켜기/)
})

test("Radeon helper 적용 오류는 게임 설정 변경 전에 최종 상태와 함께 반환한다", async () => {
  let gameWrites = 0
  const manager = createRadeonManager({
    runHelper: async () => helperResult({
      reason: "Anti-Lag 적용 실패 / 이전 3D 설정을 복원했습니다",
    }),
    setGameVerticalSync: async () => {
      gameWrites += 1
    },
  })

  await assert.rejects(() => manager.apply(), /최종 Radeon 3D 상태/)
  assert.equal(gameWrites, 0)
})

test("Radeon helper 목표 미달은 게임 설정 변경 전에 거부한다", async () => {
  let gameWrites = 0
  const manager = createRadeonManager({
    runHelper: async () => helperResult({
      allMet: false,
      reason: null,
      goals: [{
        key: "antiLagOn",
        label: "Radeon Anti-Lag 켜기",
        supported: true,
        met: false,
        currentValue: "끄기",
      }],
    }),
    setGameVerticalSync: async () => {
      gameWrites += 1
    },
  })

  await assert.rejects(() => manager.apply(), /목표 적용 검증 실패/)
  assert.equal(gameWrites, 0)
})

test("Radeon helper 비정상 종료 시 비호환 드라이버 모드로 재시도하지 않는다", async () => {
  const calls = []
  await assert.rejects(
    () => runRadeonHelper(async args => {
      calls.push(args)
      const error = new Error("helper crash")
      error.code = 3221225477
      throw error
    }, true),
    /종료 코드 0xC0000005/,
  )
  assert.deepEqual(calls, [["--apply"]])
})

test("Radeon helper가 결과 출력 후 종료되면 JSON 결과를 복구한다", async () => {
  const error = new Error("helper exit")
  error.code = 3221225477
  error.stdout = JSON.stringify(helperResult())

  const result = await runRadeonHelper(async () => {
    throw error
  }, false)

  assert.equal(result.detected, true)
})

test("Radeon helper가 출력 없이 종료되면 종료 코드를 안내한다", async () => {
  await assert.rejects(
    () => runRadeonHelper(async () => {
      const error = new Error("helper exit")
      error.code = 3221225477
      throw error
    }, false),
    /종료 코드 0xC0000005/,
  )
})

test("Radeon helper는 튜닝 미접근 계약이 없으면 결과를 거부한다", async () => {
  await assert.rejects(
    () => runRadeonHelper(async () => ({
      stdout: JSON.stringify(helperResult({ tuningAccessed: undefined })),
    }), false),
    /언더볼팅 보호 계약/,
  )
})

test("Radeon helper는 결과 출력 후 정적 종료 정리 없이 프로세스를 끝낸다", () => {
  assert.match(nativeSource, /std::fflush\(nullptr\)/)
  assert.match(nativeSource, /ExitProcess\(static_cast<UINT>\(exitCode\)\)/)
  assert.match(nativeSource, /std::vector<GpuSnapshot> snapshots/)
  assert.match(nativeSource, /writeTargets\(settings, snapshot, true/)
})
