import assert from "node:assert/strict"
import test from "node:test"
import {
  createRadeonManager,
  normalizeRadeonResult,
  runRadeonHelperWithFallback,
} from "../src/radeon.mjs"

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
    reason: null,
    ...overrides,
  }
}

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

test("Radeon helper 비정상 종료 시 레거시 드라이버 모드로 재시도한다", async () => {
  const calls = []
  const result = await runRadeonHelperWithFallback(async args => {
    calls.push(args)
    if (calls.length === 1) throw new Error("helper crash")
    return { stdout: JSON.stringify(helperResult()) }
  }, true)

  assert.equal(result.detected, true)
  assert.deepEqual(calls, [
    ["--apply"],
    ["--legacy-driver", "--apply"],
  ])
})
