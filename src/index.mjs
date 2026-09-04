import { readFile } from "node:fs/promises"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"
import { createAffinityManager } from "./affinity.mjs"
import { createMemoryManager } from "./memory.mjs"
import {
  ensureFastPingForPrimaryInterface,
  ensureTcpAutoTuningNormal,
  printFastPingStatus,
  printTcpAutoTuningStatus,
  restartPrimaryNetworkInterface,
} from "./network.mjs"

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const config = JSON.parse(await readFile(join(root, "config.json"), "utf8"))
const statePath = join(root, "runtime-state.json")
const applyChanges = process.argv.includes("--apply")
const quiet = process.argv.includes("--quiet")
const lastCoreMode = process.argv.includes("--last-core")
const passiveMode = process.argv.includes("--passive")
const resetAllMode = process.argv.includes("--reset-all")
const memoryCleanerEnabled = process.argv.includes("--memory-cleaner")
const memoryStatusMode = process.argv.includes("--memory-status")
const nvidiaStatusMode = process.argv.includes("--nvidia-status")
const nvidiaApplyMode = process.argv.includes("--nvidia-apply")
const fastPingStatusMode = process.argv.includes("--fast-ping-status")
const fastPingRestartMode = process.argv.includes("--fast-ping-restart")
const tcpAutoTuningStatusMode = process.argv.includes("--tcp-auto-tuning-status")
const nicRssStatusMode = process.argv.includes("--nic-rss-status")
const nicRssApplyMode = process.argv.includes("--nic-rss-apply")
const nicRssRestoreMode = process.argv.includes("--nic-rss-restore")
const selfTestMode = process.argv.includes("--self-test")

if (lastCoreMode && passiveMode) {
  throw new Error("Choose only one mode: --last-core or --passive.")
}
if (resetAllMode && (lastCoreMode || passiveMode)) {
  throw new Error("--reset-all cannot be combined with another mode.")
}
if (fastPingRestartMode && !applyChanges) {
  throw new Error("--fast-ping-restart requires --apply.")
}
if ((nicRssApplyMode || nicRssRestoreMode) && !applyChanges) {
  throw new Error("--nic-rss-apply and --nic-rss-restore require --apply.")
}

if (nvidiaStatusMode || nvidiaApplyMode) {
  const {
    applyNvidiaProfileGoals,
    checkNvidiaProfile,
    printNvidiaProfileStatus,
  } = await import("./nvidia.mjs")
  const result = nvidiaApplyMode
    ? await applyNvidiaProfileGoals(config.gameExecutable)
    : await checkNvidiaProfile(config.gameExecutable)
  printNvidiaProfileStatus(result)
  if (nvidiaApplyMode) console.log("NVIDIA 마비노기 최적화 목표 적용 및 검증 완료")
  process.exit(0)
}

if (fastPingStatusMode || fastPingRestartMode) {
  const result = await ensureFastPingForPrimaryInterface({
    applyChanges,
    restartAfterApply: true,
  })
  if (fastPingRestartMode && !result.restarted) {
    result.restart = await restartPrimaryNetworkInterface(result.current)
    result.restarted = true
  }
  printFastPingStatus(result)
  process.exit(0)
}

if (tcpAutoTuningStatusMode) {
  const result = await ensureTcpAutoTuningNormal({ applyChanges })
  printTcpAutoTuningStatus(result)
  process.exit(0)
}

if (nicRssStatusMode || nicRssApplyMode || nicRssRestoreMode) {
  const {
    applyNicRssAffinity,
    getNicRssAffinityStatus,
    printNicRssAffinityStatus,
    restoreNicRssAffinity,
  } = await import("./nic.mjs")
  if (nicRssRestoreMode) {
    const result = await restoreNicRssAffinity({ applyChanges })
    console.log(
      result.restored
        ? `NIC RSS affinity: 원본 복원 및 인터페이스 재시작 완료`
        : `NIC RSS affinity: ${result.reason}`,
    )
    process.exit(0)
  }
  const result = nicRssApplyMode
    ? await applyNicRssAffinity({ applyChanges })
    : await getNicRssAffinityStatus()
  printNicRssAffinityStatus(result)
  process.exit(0)
}

let stopping = false

const affinity = await createAffinityManager({
  config,
  statePath,
  applyChanges,
  quiet,
  lastCoreMode,
  passiveMode,
})
const memory = createMemoryManager({
  config,
  applyChanges,
  isGameActive: affinity.isGameActive,
  isStopping: () => stopping,
})

async function stop(signal) {
  if (stopping) return
  stopping = true
  memory.stop()
  if (!quiet) console.log(`\n[stop] ${signal}; restoring original affinities.`)
  const gameWasActive = affinity.isGameActive()
  await affinity.restoreAll()
  if (quiet && gameWasActive) console.log("마비노기 프레임 부스트 Off")
  process.exit(0)
}

process.on("SIGINT", () => void stop("SIGINT"))
process.on("SIGTERM", () => void stop("SIGTERM"))

if (!quiet) affinity.printStartupSummary()

if (selfTestMode) {
  affinity.selfTest()
  process.exit(0)
}

if (resetAllMode) {
  await affinity.resetAllAffinities()
  process.exit(0)
}

if (memoryStatusMode) {
  memory.printStatus()
  process.exit(0)
}

const tcpAutoTuning = await ensureTcpAutoTuningNormal({ applyChanges })
const fastPing = await ensureFastPingForPrimaryInterface({
  applyChanges,
  restartAfterApply: true,
})
if (!quiet) {
  printTcpAutoTuningStatus(tcpAutoTuning)
  printFastPingStatus(fastPing)
}

await affinity.recover()

if (memoryCleanerEnabled) {
  memory.start()
  if (!quiet) console.log(`[memory-cleaner] armed; ${memory.getStartupDescription()}`)
}

while (!stopping) {
  try {
    const event = await affinity.tick()
    if (event === "exited") memory.resetForGameExit()
  } catch (error) {
    console.error(`[poll-error] ${error.stack ?? error.message}`)
  }
  await new Promise(resolve => setTimeout(resolve, config.pollIntervalMs))
}
