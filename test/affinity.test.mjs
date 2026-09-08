import test from "node:test"
import assert from "node:assert/strict"
import { readFile } from "node:fs/promises"
import {
  buildCpuHalfMasks,
  buildCpuTopologyMasks,
  defaultGamePhysicalCoreCount,
  hasLiveAppliedAffinityEntries,
  matchesGameProcess,
} from "../src/affinity.mjs"

const gamePathConfig = {
  gameExecutable: "C:\\Nexon\\Mabinogi\\Client.exe",
  gameExecutableName: "Client.exe",
  gameDirectoryNames: ["Mabinogi", "Mabinogi_Test", "마비노기", "Nexon"],
}

test("정식·테스트 마비노기 및 Nexon 폴더의 Client.exe를 게임으로 인식한다", () => {
  for (const path of [
    "C:\\Nexon\\Mabinogi\\Client.exe",
    "C:\\Nexon\\Mabinogi_Test\\Client.exe",
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

test("4코어 비하이브리드 CPU는 백그라운드 1코어를 남기고 게임에 3코어를 준다", () => {
  const cpuSets = Array.from({ length: 8 }, (_, logicalProcessorIndex) => ({
    group: 0,
    logicalProcessorIndex,
    coreIndex: Math.floor(logicalProcessorIndex / 2),
    efficiencyClass: 0,
  }))

  assert.deepEqual(buildCpuTopologyMasks(cpuSets, 8), {
    source: "windows-cpu-sets",
    allMask: 0xffn,
    gameMask: 0xfcn,
    backgroundMask: 0x03n,
    alternateGameMask: 0x03n,
    alternateBackgroundMask: 0xfcn,
    lastPerformanceCoreMask: 0xc0n,
    gameCpuIndexes: [2, 3, 4, 5, 6, 7],
    backgroundCpuIndexes: [0, 1],
    physicalCoreCount: 4,
    performanceCoreCount: 4,
    efficiencyCoreCount: 0,
    gameCoreCount: 3,
    defaultGameCoreCount: 3,
    maxGameCoreCount: 3,
    hybrid: false,
  })
})

test("게임 기본 몫은 절반과 4코어 중 큰 값이며 백그라운드 1코어를 남긴다", () => {
  assert.equal(defaultGamePhysicalCoreCount(2), 1)
  assert.equal(defaultGamePhysicalCoreCount(4), 3)
  assert.equal(defaultGamePhysicalCoreCount(5), 4)
  assert.equal(defaultGamePhysicalCoreCount(6), 4)
  assert.equal(defaultGamePhysicalCoreCount(8), 4)
  assert.equal(defaultGamePhysicalCoreCount(10), 5)
})

test("사용자가 선택한 물리 코어 수만큼 SMT 스레드를 함께 게임에 배정한다", () => {
  const cpuSets = Array.from({ length: 12 }, (_, logicalProcessorIndex) => ({
    group: 0,
    logicalProcessorIndex,
    coreIndex: Math.floor(logicalProcessorIndex / 2),
    efficiencyClass: 0,
  }))

  const allocation = buildCpuTopologyMasks(cpuSets, 12, 5)

  assert.equal(allocation.gameCoreCount, 5)
  assert.deepEqual(allocation.gameCpuIndexes, [2, 3, 4, 5, 6, 7, 8, 9, 10, 11])
  assert.deepEqual(allocation.backgroundCpuIndexes, [0, 1])
})

test("CPU 코어 선택 UI와 IPC가 영구 설정 경로에 연결된다", async () => {
  const [affinitySource, mainSource, preloadSource, appSource, styleSource] = await Promise.all([
    readFile(new URL("../src/affinity.mjs", import.meta.url), "utf8"),
    readFile(new URL("../electron/main.mjs", import.meta.url), "utf8"),
    readFile(new URL("../electron/preload.cjs", import.meta.url), "utf8"),
    readFile(new URL("../web/App.svelte", import.meta.url), "utf8"),
    readFile(new URL("../web/styles.css", import.meta.url), "utf8"),
  ])

  assert.match(mainSource, /game-core-setting\.json/)
  assert.match(mainSource, /optimization:set-game-cpu-core-count/)
  assert.match(mainSource, /optimization:refresh-game-cpu-core-setting/)
  assert.match(mainSource, /describeCpuTopologyFailure/)
  assert.match(mainSource, /failureReason/)
  assert.match(mainSource, /command: "set-game-core-count"/)
  assert.match(preloadSource, /setGameCpuCoreCount/)
  assert.match(preloadSource, /refreshGameCpuCoreSetting/)
  assert.match(affinitySource, /topologyError: \{/)
  assert.match(affinitySource, /message: error\?\.message/)
  assert.match(appSource, /마비노기 CPU 우선 점유 비율 설정/)
  assert.match(appSource, /gameCpuCoreOptions\(\)/)
  assert.match(appSource, /gameCpuCoreColor\(coreCount\)/)
  assert.match(appSource, /defaultGameCoreCount/)
  assert.match(appSource, /class:allocated=/)
  assert.match(appSource, /class:applying=\{gameCpuCoreAction\}/)
  assert.doesNotMatch(appSource, /마비노기에 물리 코어 \$\{gameCoreCount\}개를 우선 배정합니다/)
  assert.match(appSource, /gameCpuUnavailableCores\(\)/)
  assert.match(appSource, /선택 불가 코어/)
  assert.match(appSource, /코어 개수가 많을수록 마비노기가 더 많은 CPU를 활용하지만/)
  assert.match(appSource, /retryGameCpuTopology/)
  assert.match(appSource, /다시 확인/)
  assert.match(appSource, /failureDetail/)
  assert.match(styleSource, /\.game-cpu-core-controls/)
  assert.match(styleSource, /\.cpu-topology-unavailable/)
  assert.match(styleSource, /flex: 1 1 0/)
  assert.match(styleSource, /\.game-cpu-core-controls\.applying/)
  assert.match(styleSource, /var\(--allocation-color\)/)
  assert.match(styleSource, /\.game-cpu-unavailable/)
})

test("하이브리드 CPU는 E-core를 제외하고 최소 4개 P-core를 게임에 준다", () => {
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

  assert.equal(allocation.gameMask, 0x0ff0n)
  assert.equal(allocation.backgroundMask, 0xff00fn)
  assert.equal(allocation.alternateGameMask, 0x000fn)
  assert.equal(allocation.alternateBackgroundMask, 0xffff0n)
  assert.deepEqual(allocation.gameCpuIndexes, [4, 5, 6, 7, 8, 9, 10, 11])
  assert.deepEqual(allocation.backgroundCpuIndexes, [0, 1, 2, 3, 12, 13, 14, 15, 16, 17, 18, 19])
  assert.equal(allocation.performanceCoreCount, 6)
  assert.equal(allocation.efficiencyCoreCount, 8)
  assert.equal(allocation.physicalCoreCount, 14)
  assert.equal(allocation.gameCoreCount, 4)
  assert.equal(allocation.defaultGameCoreCount, 4)
  assert.equal(allocation.maxGameCoreCount, 5)
  assert.equal(allocation.hybrid, true)
})

test("P-core 수가 홀수여도 최소 4코어 규칙과 SMT 묶음을 유지한다", () => {
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

  assert.deepEqual(allocation.gameCpuIndexes, [2, 3, 4, 5, 6, 7, 8, 9])
  assert.deepEqual(allocation.backgroundCpuIndexes, [0, 1, 10, 11, 12, 13])
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
