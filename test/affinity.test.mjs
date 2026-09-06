import test from "node:test"
import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import {
  buildCpuHalfMasks,
  buildCpuTopologyMasks,
  hasLiveAppliedAffinityEntries,
  matchesGameProcess,
} from "../src/affinity.mjs"

const gamePathConfig = {
  gameExecutable: "C:\\Nexon\\Mabinogi\\Client.exe",
  gameExecutableName: "Client.exe",
  gameDirectoryNames: ["Mabinogi", "마비노기", "Nexon"],
}

test("영문·한글 마비노기 및 Nexon 폴더의 Client.exe를 게임으로 인식한다", () => {
  for (const path of [
    "C:\\Nexon\\Mabinogi\\Client.exe",
    "D:\\마비노기\\Client.exe",
    "D:\\Nexon\\Client.exe",
  ]) {
    assert.equal(matchesGameProcess({ name: "Client.exe", path }, gamePathConfig), true)
  }
})

test("허용 폴더 밖의 같은 이름 실행 파일은 마비노기로 인식하지 않는다", () => {
  assert.equal(
    matchesGameProcess(
      { name: "Client.exe", path: "D:\\OtherGame\\Client.exe" },
      gamePathConfig,
    ),
    false,
  )
})

test("허용 폴더가 아니어도 같은 디렉토리에 Mabinogi.exe가 있으면 게임으로 인식한다", () => {
  let checkedPath
  const matched = matchesGameProcess(
    { name: "Client.exe", path: "D:\\CustomGame\\Client.exe" },
    gamePathConfig,
    path => {
      checkedPath = path
      return true
    },
  )

  assert.equal(matched, true)
  assert.equal(checkedPath, "D:\\CustomGame\\Mabinogi.exe")
})

test("CPU 절반 마스크를 논리 CPU 수에 맞게 동적으로 계산한다", () => {
  assert.deepEqual(buildCpuHalfMasks(4), {
    half: 2,
    allMask: 0xfn,
    lowerHalfMask: 0x3n,
    upperHalfMask: 0xcn,
  })
  assert.deepEqual(buildCpuHalfMasks(8), {
    half: 4,
    allMask: 0xffn,
    lowerHalfMask: 0x0fn,
    upperHalfMask: 0xf0n,
  })
  assert.deepEqual(buildCpuHalfMasks(12), {
    half: 6,
    allMask: 0xfffn,
    lowerHalfMask: 0x03fn,
    upperHalfMask: 0xfc0n,
  })
  assert.deepEqual(buildCpuHalfMasks(16), {
    half: 8,
    allMask: 0xffffn,
    lowerHalfMask: 0x00ffn,
    upperHalfMask: 0xff00n,
  })
})

test("지원하지 않는 논리 CPU 수는 마스크를 만들지 않는다", () => {
  assert.throws(() => buildCpuHalfMasks(2), /Unsupported logical CPU count/)
  assert.throws(() => buildCpuHalfMasks(7), /Unsupported logical CPU count/)
  assert.throws(() => buildCpuHalfMasks(54), /Unsupported logical CPU count/)
})

test("동일 성능 코어는 물리 코어와 SMT 스레드를 함께 절반으로 나눈다", () => {
  const cpuSets = Array.from({ length: 8 }, (_, logicalProcessorIndex) => ({
    group: 0,
    logicalProcessorIndex,
    coreIndex: Math.floor(logicalProcessorIndex / 2),
    efficiencyClass: 0,
  }))

  assert.deepEqual(buildCpuTopologyMasks(cpuSets, 8), {
    source: "windows-cpu-sets",
    allMask: 0xffn,
    gameMask: 0xf0n,
    backgroundMask: 0x0fn,
    alternateGameMask: 0x0fn,
    alternateBackgroundMask: 0xf0n,
    lastPerformanceCoreMask: 0xc0n,
    gameCpuIndexes: [4, 5, 6, 7],
    backgroundCpuIndexes: [0, 1, 2, 3],
    performanceCoreCount: 4,
    efficiencyCoreCount: 0,
    hybrid: false,
  })
})

test("하이브리드 CPU는 P-core 절반만 게임에 주고 나머지 P/E-core를 백그라운드에 준다", () => {
  const performanceSets = Array.from({ length: 12 }, (_, logicalProcessorIndex) => ({
    group: 0,
    logicalProcessorIndex,
    coreIndex: Math.floor(logicalProcessorIndex / 2),
    efficiencyClass: 8,
  }))
  const efficiencySets = Array.from({ length: 8 }, (_, index) => ({
    group: 0,
    logicalProcessorIndex: index + 12,
    coreIndex: index + 6,
    efficiencyClass: 0,
  }))

  const allocation = buildCpuTopologyMasks([...performanceSets, ...efficiencySets], 20)

  assert.equal(allocation.gameMask, 0x0fc0n)
  assert.equal(allocation.backgroundMask, 0xff03fn)
  assert.equal(allocation.alternateGameMask, 0x003fn)
  assert.equal(allocation.alternateBackgroundMask, 0xfffc0n)
  assert.deepEqual(allocation.gameCpuIndexes, [6, 7, 8, 9, 10, 11])
  assert.deepEqual(allocation.backgroundCpuIndexes, [0, 1, 2, 3, 4, 5, 12, 13, 14, 15, 16, 17, 18, 19])
  assert.equal(allocation.performanceCoreCount, 6)
  assert.equal(allocation.efficiencyCoreCount, 8)
  assert.equal(allocation.hybrid, true)
})

test("P-core 수가 홀수면 게임 몫을 올림하고 SMT 스레드를 분리하지 않는다", () => {
  const performanceSets = Array.from({ length: 10 }, (_, logicalProcessorIndex) => ({
    group: 0,
    logicalProcessorIndex,
    coreIndex: Math.floor(logicalProcessorIndex / 2),
    efficiencyClass: 4,
  }))
  const efficiencySets = Array.from({ length: 4 }, (_, index) => ({
    group: 0,
    logicalProcessorIndex: index + 10,
    coreIndex: index + 5,
    efficiencyClass: 0,
  }))

  const allocation = buildCpuTopologyMasks([...performanceSets, ...efficiencySets], 14)

  assert.deepEqual(allocation.gameCpuIndexes, [4, 5, 6, 7, 8, 9])
  assert.deepEqual(allocation.backgroundCpuIndexes, [0, 1, 2, 3, 10, 11, 12, 13])
})

test("입력 장치와 매크로 엔진은 지연 민감 프로세스로 분류한다", async () => {
  const config = JSON.parse(await readFile(new URL("../config.json", import.meta.url), "utf8"))
  const excludeNames = new Set(config.excludeNames.map(name => name.toLowerCase()))
  const excludePatterns = config.excludeNamePatterns.map(pattern => new RegExp(pattern, "i"))
  const latencyNames = new Set(config.latencySensitiveNames.map(name => name.toLowerCase()))
  const latencyPatterns = config.latencySensitiveNamePatterns.map(pattern => new RegExp(pattern, "i"))
  const inputEngineNames = [
    "iCUE.exe",
    "lghub_agent.exe",
    "logioptionsplus_agent.exe",
    "NGenuity.exe",
    "RazerAppEngine.exe",
    "Razer Synapse Service Process.exe",
    "ROCCAT_Swarm_Monitor.exe",
    "SwarmHW_Service.exe",
    "SteelSeriesEngine.exe",
    "Wootomation.exe",
    "wooting-double-movement.exe",
    "Glorious Core.exe",
    "MasterPlusApp.exe",
    "reWASDEngine.exe",
    "reWASDService.exe",
    "XMouseButtonControl.exe",
    "PowerToys.KeyboardManagerEngine.exe",
    "AutoHotkeyU64.exe",
    "AutoHotkey64_UIA.exe",
    "StreamDeck.exe",
    "turbo-key-helper.exe",
  ]

  for (const name of inputEngineNames) {
    const excluded = excludeNames.has(name.toLowerCase())
      || excludePatterns.some(pattern => pattern.test(name))
    const latencySensitive = latencyNames.has(name.toLowerCase())
      || latencyPatterns.some(pattern => pattern.test(name))
    assert.equal(excluded, false, `${name} should not use the full CPU range`)
    assert.equal(latencySensitive, true, `${name} should use non-game P-cores`)
  }
})

test("입력 엔진과 무관한 장치 프로그램 UI와 부가 프로세스는 제외하지 않는다", async () => {
  const config = JSON.parse(await readFile(new URL("../config.json", import.meta.url), "utf8"))
  const excludeNames = new Set(config.excludeNames.map(name => name.toLowerCase()))
  const excludePatterns = config.excludeNamePatterns.map(pattern => new RegExp(pattern, "i"))
  const auxiliaryNames = [
    "node.exe",
    "lghub.exe",
    "lghub_updater.exe",
    "Razer Central.exe",
    "RzSDKServer.exe",
    "Wootility.exe",
    "SteelSeriesGG.exe",
    "SteelSeriesGGEZ.exe",
    "SteelSeriesPrism.exe",
    "SteelSeriesSonar.exe",
  ]

  for (const name of auxiliaryNames) {
    const excluded = excludeNames.has(name.toLowerCase())
      || excludePatterns.some(pattern => pattern.test(name))
    assert.equal(excluded, false, `${name} should remain adjustable`)
  }
})

test("적용 기록이 없으면 프로세스 조회 없이 종료 확인을 생략한다", async () => {
  const result = await hasLiveAppliedAffinityEntries([], {
    processStartReader: () => {
      throw new Error("호출되면 안 됨")
    },
  })

  assert.equal(result, false)
})

test("PID와 시작 시각 및 현재 마스크가 모두 일치할 때만 적용 상태로 판정한다", async () => {
  const entries = [{
    pid: 1234,
    startTime: "2026-09-05T17:00:00.000Z",
    appliedMask: "0xff",
  }]
  const processStartReader = () => Date.parse("2026-09-05T17:00:00.000Z")

  assert.equal(await hasLiveAppliedAffinityEntries(entries, {
    processStartReader,
    affinityReader: () => 0xffn,
  }), true)
  assert.equal(await hasLiveAppliedAffinityEntries(entries, {
    processStartReader,
    affinityReader: () => 0xffffn,
  }), false)
})
