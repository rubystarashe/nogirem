import { execFile, spawn } from "node:child_process"
import { existsSync, unlinkSync } from "node:fs"
import { copyFile, mkdir, open as openFile, readFile, rename, stat, unlink, writeFile } from "node:fs/promises"
import { cpus, tmpdir } from "node:os"
import { dirname, join } from "node:path"
import { setTimeout as delay } from "node:timers/promises"
import { promisify } from "node:util"
import { fileURLToPath } from "node:url"
import { randomUUID } from "node:crypto"
import { app, BrowserWindow, dialog, ipcMain, Menu, shell, Tray } from "electron"
import updaterPackage from "electron-updater"
import {
  ensureFastPingForPrimaryInterface,
  ensureTcpAutoTuningNormal,
} from "../src/network.mjs"
import {
  applyNicRssAffinity,
  buildNicRssAffinityPlan,
  getNicRssAffinityStatus,
  isNicRssAffinityOptimized,
  restoreNicRssAffinity,
} from "../src/nic.mjs"
import { createMemoryManager } from "../src/memory.mjs"
import {
  applyInstalledDxvk,
  detectDxvkRendererFromLog,
  getDxvkDeploymentStatus,
  getDxvkReleases,
  getInstalledDxvk,
  installDxvkVersion,
} from "../src/dxvk.mjs"
import { getLatestMuoStatus } from "../src/muo-status.mjs"
import { writeJsonAtomic } from "../src/atomic-json.mjs"
import { readRuntimeStatusJson as readRuntimeStatusJsonFile } from "../src/runtime-status.mjs"
import { advanceDownloadProgress } from "../src/update-progress.mjs"
import { getYouTubeChannelProfile } from "../src/youtube-channel.mjs"
import { assessExitConfirmation } from "../src/exit-confirmation.mjs"
import { getGameDirectoryNames, resolveCpuAllocation } from "../src/affinity.mjs"
import {
  defaultTurboKeyCodes,
  normalizeTurboKeyCodes,
  normalizeTurboKeyIntervalMs,
} from "../src/turbo-key-settings.mjs"
import {
  getTurboKeyHelperInstallation,
  installTurboKeyHelper,
  removeTurboKeyHelper,
  turboKeyHelperVersion,
} from "../src/turbo-key-installer.mjs"

const { autoUpdater } = updaterPackage
const execFileAsync = promisify(execFile)
const root = dirname(dirname(fileURLToPath(import.meta.url)))
const preloadPath = join(root, "electron", "preload.cjs")
const characterGuidePreloadPath = join(root, "electron", "character-guide-preload.cjs")
const dxvkManagerPreloadPath = join(root, "electron", "dxvk-manager-preload.cjs")
const dxvkGuidePreloadPath = join(root, "electron", "dxvk-guide-preload.cjs")
const iconPath = join(root, "icon.ico")
const pausedTrayIconPath = join(root, "icon-paused.png")
const characterSimplificationFileName = "주변캐릭터간소화프레임제한해제.muo"
const creatorChannelUrl = "https://www.youtube.com/channel/UCb7m0UV734CHm78Mb0zEBHg"
const directDonationUrl = "https://thedirectdonation.org/"
const operationPolicyUrl = "https://mabinogi.nexon.com/page/archive/guide_view.asp?id=4889849&num=7&playtarget=1"
const startupTrayTaskName = "Mabinogi Rem Booster Startup"
const startupTrayLaunch = process.argv.includes("--startup-tray")
const applicationUpdateStallTimeoutMs = 45_000
const primaryRendererUnresponsiveTimeoutMs = 5_000
const primaryWindowRevealTimeoutMs = 8_000
const trayMenuCloseDelayMs = 75
const primaryWindowFocusRetryDelayMs = 150

function writeStartupLog(message) {
  globalThis.__nogiremWriteStartupLog?.("INFO", message)
}
const conflictingProgramDefinitions = [
  {
    name: "ISLC",
    executables: ["islc.exe", "intelligent standby list cleaner islc.exe"],
  },
  { name: "Process Lasso", executables: ["processlasso.exe", "processgovernor.exe"] },
]
const config = JSON.parse(await readFile(join(root, "config.json"), "utf8"))
let primaryWindow = null
let characterGuideWindow = null
let dxvkManagerWindow = null
let dxvkGuideWindow = null
let applicationTray = null
let closeRequestPending = false
let applicationExitInProgress = false
let primaryWindowFocusPending = false
let primaryWindowFocusTimer = null
let primaryWindowTrayRestoreTimer = null
let primaryVisualActivityTimer = null
let primaryWindowSkippedFromTaskbar = false
let primaryWindowRevealFrameTimer = null
let primaryWindowRevealWatchdogTimer = null
let primaryWindowRevealStarted = false
let focusRequestMonitor = null
let focusRequestReading = false
let lastFocusRequestAt = 0
let lastInstallerCloseRequestAt = 0
let characterGuideDrag = null
let dxvkGuideDrag = null
let dxvkUpdatePromise = null
let dxvkRuntimeStatus = {
  state: "checking",
  latestVersion: null,
  error: null,
}
let dxvkRuntimeCheckPromise = null
let dxvkReleasesCache = []
let dxvkReleasesCheckedAt = 0
let dxvkReleasesCheckPromise = null
let dxvkReleasesCacheError = null
let dxvkRuntimeRefreshTimer = null
let applicationUpdateStartupTimer = null
let applicationUpdateCheckTimer = null
let applicationUpdateCompletionTimer = null
let applicationUpdateStallTimer = null
let applicationUpdateDownloadStalled = false
let applicationUpdateCheckPromise = null
let applicationUpdaterConfigured = false
let applicationUpdateState = {
  phase: "idle",
  percent: 0,
  version: null,
  error: null,
}
let activeMabinogiExecutablePath = config.gameExecutable
let gamePathStateMonitor = null
let gamePathStateReading = false
let creatorChannelProfilePromise = null
let turboKeyProcess = null
let turboKeyInstallationCache = null
let primaryRendererRecoveryMode = false
let primaryRendererRecoveryInProgress = false
let sandboxCompatibilityRelaunching = false
let primaryRendererUnresponsiveTimer = null
let primaryRendererRecoveryResetTimer = null
const dxvkReleaseCacheDurationMs = 6 * 60 * 1000

const instanceDirectory = join(app.getPath("userData"), "instance")
const primaryInstancePath = join(instanceDirectory, "primary.json")
const focusRequestPath = join(instanceDirectory, "focus-request.json")
const focusAcknowledgementPath = join(instanceDirectory, "focus-acknowledgement.json")
const installerCloseRequestPath = join(instanceDirectory, "installer-close-request")

function getTurboKeyPaths() {
  const directory = join(app.getPath("userData"), "turbo-key")
  return {
    directory,
    settingsPath: join(directory, "settings.json"),
    statusPath: join(directory, "status.json"),
    controlPath: join(directory, "control.json"),
  }
}

function serializeError(error) {
  return {
    name: error?.name ?? "Error",
    message: error?.message ?? String(error),
  }
}

function notifyApplicationUpdateState() {
  if (
    !primaryWindow
    || primaryWindow.isDestroyed()
    || primaryWindow.webContents.isDestroyed()
  ) return
  primaryWindow.webContents.send("application:update-state-changed", applicationUpdateState)
}

function setApplicationUpdateState(nextState) {
  applicationUpdateState = {
    ...applicationUpdateState,
    ...nextState,
  }
  notifyApplicationUpdateState()
}

function clearApplicationUpdateCompletionTimer() {
  clearTimeout(applicationUpdateCompletionTimer)
  applicationUpdateCompletionTimer = null
}

function clearApplicationUpdateStallTimer() {
  clearTimeout(applicationUpdateStallTimer)
  applicationUpdateStallTimer = null
}

function armApplicationUpdateStallTimer() {
  clearApplicationUpdateStallTimer()
  applicationUpdateStallTimer = setTimeout(() => {
    applicationUpdateStallTimer = null
    if (applicationUpdateState.phase !== "downloading") return
    applicationUpdateDownloadStalled = true
    setApplicationUpdateState({
      phase: "error",
      percent: 0,
      error: {
        name: "UpdateDownloadStalled",
        message: "업데이트 다운로드가 응답하지 않아 중단했습니다. 버전을 눌러 다시 시도하세요",
      },
    })
  }, applicationUpdateStallTimeoutMs)
}

function configureApplicationUpdater() {
  if (applicationUpdaterConfigured || !app.isPackaged) return
  applicationUpdaterConfigured = true
  autoUpdater.autoDownload = true
  autoUpdater.autoInstallOnAppQuit = false

  autoUpdater.on("checking-for-update", () => {
    clearApplicationUpdateCompletionTimer()
    clearApplicationUpdateStallTimer()
    applicationUpdateDownloadStalled = false
    setApplicationUpdateState({
      phase: "checking",
      percent: 0,
      error: null,
    })
  })
  autoUpdater.on("update-available", info => {
    clearApplicationUpdateCompletionTimer()
    applicationUpdateDownloadStalled = false
    armApplicationUpdateStallTimer()
    setApplicationUpdateState({
      phase: "downloading",
      percent: 0,
      version: info?.version ?? null,
      error: null,
    })
  })
  autoUpdater.on("update-not-available", () => {
    clearApplicationUpdateCompletionTimer()
    clearApplicationUpdateStallTimer()
    applicationUpdateDownloadStalled = false
    setApplicationUpdateState({
      phase: "idle",
      percent: 0,
      version: null,
      error: null,
    })
  })
  autoUpdater.on("download-progress", progress => {
    if (applicationUpdateDownloadStalled) return
    armApplicationUpdateStallTimer()
    const previousPercent = applicationUpdateState.phase === "downloading"
      ? applicationUpdateState.percent
      : 0
    const percent = advanceDownloadProgress(previousPercent, progress?.percent)
    setApplicationUpdateState({
      phase: "downloading",
      percent,
      error: null,
    })
  })
  autoUpdater.on("update-downloaded", info => {
    clearApplicationUpdateCompletionTimer()
    clearApplicationUpdateStallTimer()
    applicationUpdateDownloadStalled = false
    setApplicationUpdateState({
      phase: "downloading",
      percent: 100,
      version: info?.version ?? applicationUpdateState.version,
      error: null,
    })
    applicationUpdateCompletionTimer = setTimeout(() => {
      applicationUpdateCompletionTimer = null
      setApplicationUpdateState({
        phase: "downloaded",
        percent: 100,
        error: null,
      })
    }, 350)
  })
  autoUpdater.on("error", error => {
    clearApplicationUpdateCompletionTimer()
    clearApplicationUpdateStallTimer()
    applicationUpdateDownloadStalled = false
    console.error("앱 업데이트 확인 실패", error)
    setApplicationUpdateState({
      phase: "error",
      percent: 0,
      error: serializeError(error),
    })
  })
}

async function checkForApplicationUpdate() {
  if (!app.isPackaged) return applicationUpdateState
  if (applicationUpdateCheckPromise) return applicationUpdateCheckPromise
  configureApplicationUpdater()
  applicationUpdateCheckPromise = autoUpdater.checkForUpdates()
    .then(() => applicationUpdateState)
    .catch(error => {
      setApplicationUpdateState({
        phase: "error",
        percent: 0,
        error: serializeError(error),
      })
      return applicationUpdateState
    })
    .finally(() => {
      applicationUpdateCheckPromise = null
    })
  return applicationUpdateCheckPromise
}

async function installDownloadedApplicationUpdate() {
  if (!app.isPackaged || applicationUpdateState.phase !== "downloaded") return false
  applicationExitInProgress = true
  closeRequestPending = false
  const results = await Promise.allSettled([
    stopAffinityHelper(),
    stopMemoryHelper(),
    stopTurboKeyHelper(),
  ])
  for (const result of results) {
    if (result.status === "rejected") console.error(result.reason)
  }
  autoUpdater.quitAndInstall(false, true)
  return true
}

function argumentValue(name) {
  return process.argv.find(argument => argument.startsWith(`--${name}=`))
    ?.slice(name.length + 3)
}

function findConflictingPrograms(processNames = []) {
  const running = new Set(processNames.map(name => name.toLowerCase()))
  return conflictingProgramDefinitions
    .filter(program => program.executables.some(executable => running.has(executable)))
    .map(program => program.name)
}

async function readCreatorChannelProfile() {
  creatorChannelProfilePromise ??= getYouTubeChannelProfile({
    cachePath: join(app.getPath("userData"), "creator", "youtube-channel.json"),
  })
  try {
    return await creatorChannelProfilePromise
  } finally {
    creatorChannelProfilePromise = null
  }
}

function creatorPromptStatePath() {
  return join(app.getPath("userData"), "creator", "prompt.json")
}

async function readCreatorPromptDismissed() {
  const state = await readJson(creatorPromptStatePath())
  return state?.dismissed === true
}

async function dismissCreatorPrompt() {
  await writeJsonAtomic(creatorPromptStatePath(), {
    dismissed: true,
    dismissedAt: new Date().toISOString(),
  })
  return true
}

async function detectMabinogiRenderer(gameActive, gameStartTime, gameExecutablePath) {
  if (!gameActive) return { mode: "not-running", version: null }
  const startedAt = Date.parse(gameStartTime)
  if (!Number.isFinite(startedAt)) return { mode: "detecting", version: null }
  const logPath = join(dirname(gameExecutablePath), "Client_d3d9.log")
  try {
    const [content, logStat] = await Promise.all([
      readFile(logPath, "utf8"),
      stat(logPath),
    ])
    const parsed = detectDxvkRendererFromLog(content)
    const belongsToCurrentLaunch = logStat.mtimeMs >= startedAt - 5000
    if (belongsToCurrentLaunch && parsed.initialized) {
      return { mode: "vulkan", version: parsed.version }
    }
  } catch (error) {
    if (error?.code !== "ENOENT") {
      return { mode: "detecting", version: null }
    }
  }
  return {
    mode: Date.now() - startedAt >= 15000 ? "direct3d9" : "detecting",
    version: null,
  }
}

async function getCharacterSimplificationStatus() {
  const directory = join(app.getPath("documents"), "마비노기", "설정")
  try {
    const status = await getLatestMuoStatus(directory)
    return {
      applied: status.applied,
      value: status.value,
      fileName: status.fileName,
      modifiedAt: status.modifiedAt,
      error: null,
    }
  } catch (error) {
    return {
      applied: false,
      value: null,
      fileName: null,
      modifiedAt: null,
      error: serializeError(error),
    }
  }
}

async function readJson(path) {
  try {
    return JSON.parse(await readFile(path, "utf8"))
  } catch (error) {
    if (error.code === "ENOENT") return null
    throw error
  }
}

async function readRuntimeStatusJson(path) {
  return readRuntimeStatusJsonFile(path, error => {
    console.error(`손상된 런타임 상태 파일을 제거합니다: ${path}`, error)
  })
}

function isProcessRunning(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false
  try {
    process.kill(pid, 0)
    return true
  } catch (error) {
    return error?.code === "EPERM"
  }
}

async function acquireHelperLock(statusPath) {
  const lockPath = `${statusPath}.lock`
  await mkdir(dirname(lockPath), { recursive: true })
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      const handle = await openFile(lockPath, "wx")
      await handle.writeFile(JSON.stringify({
        pid: process.pid,
        startedAt: Date.now(),
      }), "utf8")
      return {
        release: async () => {
          await handle.close().catch(() => {})
          await unlink(lockPath).catch(() => {})
        },
      }
    } catch (error) {
      if (error?.code !== "EEXIST") throw error
      const existing = await readJson(lockPath).catch(() => null)
      if (isProcessRunning(existing?.pid)) {
        throw new Error(`이미 실행 중인 helper가 있습니다: PID ${existing.pid}`)
      }
      await unlink(lockPath).catch(unlinkError => {
        if (unlinkError?.code !== "ENOENT") throw unlinkError
      })
    }
  }
  throw new Error("helper 단일 실행 잠금을 가져오지 못했습니다")
}

async function fileModifiedAt(path) {
  try {
    return (await stat(path)).mtimeMs
  } catch (error) {
    if (error.code === "ENOENT") return 0
    throw error
  }
}

async function requestPrimaryWindowFocus(shouldFocus = true) {
  const request = {
    requestId: randomUUID(),
    requestedAt: Date.now(),
    requesterPid: process.pid,
    shouldFocus,
  }
  await writeJsonAtomic(focusRequestPath, request)
  return request
}

async function focusRunningPrimaryInstance(shouldFocus = true) {
  const primary = await readJson(primaryInstancePath)
  if (!Number.isInteger(primary?.pid) || primary.pid <= 0) return false
  const request = await requestPrimaryWindowFocus(shouldFocus)
  for (let attempt = 0; attempt < 12; attempt += 1) {
    await delay(100)
    const acknowledgement = await readJson(focusAcknowledgementPath)
    if (acknowledgement?.requestId === request.requestId) return true
  }
  const latestPrimary = await readJson(primaryInstancePath)
  if (
    latestPrimary?.pid === primary.pid
    && latestPrimary?.startedAt === primary.startedAt
  ) {
    await unlink(primaryInstancePath).catch(() => {})
  }
  return false
}

async function startFocusRequestMonitor() {
  await mkdir(instanceDirectory, { recursive: true })
  const existingRequest = await readJson(focusRequestPath)
  lastFocusRequestAt = existingRequest?.requestedAt ?? 0
  lastInstallerCloseRequestAt = await fileModifiedAt(installerCloseRequestPath)
  await writeJsonAtomic(primaryInstancePath, {
    pid: process.pid,
    startedAt: Date.now(),
  })
  focusRequestMonitor = setInterval(async () => {
    if (focusRequestReading) return
    focusRequestReading = true
    try {
      const installerCloseRequestAt = await fileModifiedAt(installerCloseRequestPath)
      if (
        installerCloseRequestAt > lastInstallerCloseRequestAt
        && !applicationExitInProgress
      ) {
        lastInstallerCloseRequestAt = installerCloseRequestAt
        await unlink(installerCloseRequestPath).catch(() => {})
        await finishApplicationExit("keep")
        return
      }
      const request = await readJson(focusRequestPath)
      if ((request?.requestedAt ?? 0) > lastFocusRequestAt) {
        lastFocusRequestAt = request.requestedAt
        if (request.shouldFocus !== false) focusPrimaryWindow()
        await writeJsonAtomic(focusAcknowledgementPath, {
          requestId: request.requestId,
          acknowledgedAt: Date.now(),
          primaryPid: process.pid,
        })
      }
    } catch (error) {
      console.error("기존 창 포커스 요청 확인 실패", error)
    } finally {
      focusRequestReading = false
    }
  }, 200)
}

async function waitForAffinityCommand(controlPath, durationMs) {
  const deadline = Date.now() + durationMs
  while (true) {
    const control = await readJson(controlPath)
    if (["stop", "reset", "cpu-reorder"].includes(control?.command)) {
      return control
    }
    const remaining = deadline - Date.now()
    if (remaining <= 0) return null
    await delay(Math.min(250, remaining))
  }
}

async function runAffinityHelper() {
  const statusPath = argumentValue("status-path")
  const controlPath = argumentValue("control-path")
  const affinityStatePath = argumentValue("affinity-state-path")
  const appliedMarkerPath = argumentValue("applied-marker-path")
  const gamePathStatePath = argumentValue("game-path-state-path")
  const includeNic = argumentValue("include-nic") === "true"
  const inheritedNicManaged = argumentValue("nic-managed") === "true"
  if (!statusPath || !controlPath || !affinityStatePath) {
    throw new Error("Affinity helper 제어 경로가 없습니다")
  }
  const helperLock = await acquireHelperLock(statusPath)

  let stopping = false
  let affinity = null
  let nicManaged = inheritedNicManaged
  let failure = null
  let helperAffinity = null
  let nicStatus = null
  let appliedMarker = appliedMarkerPath ? await readJson(appliedMarkerPath) : null
  let appliedMarkerRecorded = Boolean(appliedMarker)
  let recordedChangeCount = 0
  let exitAction = "keep"
  let cpuReorder = null
  const savedGameExecutablePath = (await readJson(gamePathStatePath))?.executablePath
  const previousGameExecutablePath = (await readRuntimeStatusJson(statusPath))?.gameExecutablePath
  let persistedGameExecutablePath = savedGameExecutablePath
  let gameExecutablePath = [savedGameExecutablePath, previousGameExecutablePath]
    .find(value => typeof value === "string" && value.trim())
    ?? config.gameExecutable
  const logicalCpuCount = cpus().length
  const half = logicalCpuCount / 2

  const updateAppliedMarker = async entries => {
    if (!appliedMarkerPath) return
    const mergedEntries = new Map(
      (appliedMarker?.entries ?? []).map(entry => [`${entry.pid}:${entry.startTime}`, entry]),
    )
    for (const entry of entries) {
      mergedEntries.set(`${entry.pid}:${entry.startTime}`, entry)
    }
    appliedMarker = {
      appliedAt: appliedMarker?.appliedAt ?? Date.now(),
      helperPid: process.pid,
      nicManaged: Boolean(appliedMarker?.nicManaged || nicManaged),
      entries: [...mergedEntries.values()],
    }
    await writeJsonAtomic(appliedMarkerPath, appliedMarker)
    appliedMarkerRecorded = true
  }

  const updateAppliedMarkerSafely = async entries => {
    try {
      await updateAppliedMarker(entries)
      return true
    } catch (error) {
      console.error("Affinity 적용 기록 실패, 다음 주기에 다시 시도합니다", error)
      return false
    }
  }

  const writeStatus = async values => {
    const gameActive = affinity?.isGameActive() ?? false
    const detectedGameExecutablePath = affinity?.getLatestGameExecutablePath()
    if (
      detectedGameExecutablePath
      && (
        detectedGameExecutablePath.toLowerCase() !== gameExecutablePath.toLowerCase()
        || detectedGameExecutablePath.toLowerCase()
          !== String(persistedGameExecutablePath ?? "").toLowerCase()
      )
    ) {
      gameExecutablePath = detectedGameExecutablePath
      if (gamePathStatePath) {
        await writeJsonAtomic(gamePathStatePath, {
          executablePath: gameExecutablePath,
          detectedAt: new Date().toISOString(),
          source: "running-process",
        })
        persistedGameExecutablePath = gameExecutablePath
      }
    }
    const [renderer, characterSimplification] = await Promise.all([
      detectMabinogiRenderer(
        gameActive,
        affinity?.getLatestGameStartTime(),
        gameExecutablePath,
      ),
      getCharacterSimplificationStatus(),
    ])
    return writeJsonAtomic(statusPath, {
      running: !stopping,
      gameActive,
      gameExecutablePath,
      renderer,
      characterSimplification,
      includeNic,
      nicManaged,
      backgroundCpuRange: affinity?.getCpuAllocation().backgroundCpuRange
        ?? helperAffinity?.backgroundCpuRange
        ?? `0-${half - 1}`,
      gameCpuRange: affinity?.getCpuAllocation().gameCpuRange
        ?? helperAffinity?.gameCpuRange
        ?? `${half}-${logicalCpuCount - 1}`,
      cpuTopology: affinity?.getCpuAllocation() ?? (
        helperAffinity
          ? {
              source: helperAffinity.source,
              hybrid: helperAffinity.hybrid,
              performanceCoreCount: helperAffinity.performanceCoreCount,
              efficiencyCoreCount: helperAffinity.efficiencyCoreCount,
            }
          : null
      ),
      helperPid: process.pid,
      helperAffinity,
      nicStatus,
      appliedMarkerRecorded,
      conflictingPrograms: findConflictingPrograms(affinity?.getRunningProcessNames()),
      cpuReorder,
      updatedAt: Date.now(),
      error: failure,
      ...values,
    })
  }

  const writeStatusSafely = async values => {
    try {
      await writeStatus(values)
      return true
    } catch (error) {
      console.error("Affinity 상태 기록 실패, 다음 주기에 다시 시도합니다", error)
      return false
    }
  }

  const requestStop = () => {
    stopping = true
  }
  process.on("SIGINT", requestStop)
  process.on("SIGTERM", requestStop)

  const handleCommand = async control => {
    if (!control) return false
    if (control.command === "cpu-reorder") {
      await unlink(controlPath).catch(() => {})
      cpuReorder = {
        requestId: control.requestId,
        state: "running",
        startedAt: Date.now(),
        endsAt: Date.now() + 3000,
        error: null,
      }
      await writeStatusSafely()
      try {
        const result = await affinity.performCpuReorder()
        cpuReorder = {
          ...cpuReorder,
          state: "completed",
          completedAt: Date.now(),
          result,
        }
      } catch (error) {
        cpuReorder = {
          ...cpuReorder,
          state: "failed",
          completedAt: Date.now(),
          error: serializeError(error),
        }
      }
      await writeStatusSafely()
      return false
    }
    exitAction = control.command === "reset" ? "reset" : "keep"
    stopping = true
    return true
  }

  try {
    const {
      createAffinityManager,
      pinCurrentProcessToBackgroundCpus,
    } = await import("../src/affinity.mjs")
    helperAffinity = pinCurrentProcessToBackgroundCpus()
    await unlink(affinityStatePath).catch(() => {})

    if (includeNic) {
      const nic = await applyNicRssAffinity({ applyChanges: true })
      nicManaged ||= nic.applied
      nicStatus = nic
      if (nicManaged) await updateAppliedMarkerSafely([])
    }

    affinity = await createAffinityManager({
      config,
      statePath: affinityStatePath,
      applyChanges: true,
      quiet: true,
      lastCoreMode: false,
      passiveMode: false,
      restoreOnGameExit: true,
    })
    await writeStatusSafely()

    while (!stopping) {
      const immediateCommand = await waitForAffinityCommand(controlPath, 0)
      if (await handleCommand(immediateCommand)) break
      await affinity.tick()
      const appliedChanges = affinity.getAppliedChanges()
      if (appliedChanges.length > recordedChangeCount) {
        if (await updateAppliedMarkerSafely(appliedChanges)) {
          recordedChangeCount = appliedChanges.length
        }
      }
      await writeStatusSafely()
      const delayedCommand = await waitForAffinityCommand(controlPath, config.pollIntervalMs)
      if (await handleCommand(delayedCommand)) break
    }
  } catch (error) {
    stopping = true
    failure = serializeError(error)
    await writeStatusSafely({ error: serializeError(error) })
    throw error
  } finally {
    stopping = true
    try {
      if (affinity) {
        if (exitAction === "reset") await affinity.resetAllAffinities()
        else await affinity.stopWithoutRestore()
      }
    } catch (error) {
      failure ??= serializeError(error)
    }
    if (exitAction === "reset" && nicManaged) {
      try {
        const restored = await restoreNicRssAffinity({ applyChanges: true })
        if (restored.current) {
          const target = buildNicRssAffinityPlan(restored.current, { logicalCpuCount })
          nicStatus = {
            supported: true,
            optimized: isNicRssAffinityOptimized(restored.current, target),
            rssEnabled: restored.current.enabled,
            current: restored.current,
            target,
            gameCpuOverlap: restored.current.enabled !== true
              || restored.current.maxProcessorNumber === null
              || restored.current.maxProcessorNumber >= target.gameProcessorStart,
          }
        } else {
          nicStatus = await getNicRssAffinityStatus()
        }
        nicManaged = false
      } catch (error) {
        failure ??= serializeError(error)
      }
    }
    if (exitAction === "reset" && !failure && appliedMarkerPath) {
      await unlink(appliedMarkerPath).catch(() => {})
      appliedMarker = null
      appliedMarkerRecorded = false
    }
    await writeStatusSafely({
      running: false,
      gameActive: false,
      exitAction,
      stoppedAt: Date.now(),
    })
    await helperLock.release()
  }
}

async function detectMabinogi() {
  const executableName = String(config.gameExecutableName ?? "Client.exe")
  const configuredPath = String(config.gameExecutable ?? "").replaceAll("/", "\\").toLowerCase()
  const directoryNames = getGameDirectoryNames(config)
  const script = `
$ErrorActionPreference = "SilentlyContinue"
$directoryNames = @(${directoryNames.map(quotePowerShellLiteral).join(", ")})
$found = Get-Process | Where-Object {
  ($_.ProcessName + ".exe") -ieq ${quotePowerShellLiteral(executableName)}
} | Where-Object {
  if (-not $_.Path) {
    $false
  } else {
    $path = $_.Path.Replace("/", "\\").ToLowerInvariant()
    $parentDirectoryName = [System.IO.Path]::GetFileName(
      [System.IO.Path]::GetDirectoryName($path)
    )
    $mabinogiLauncherPath = [System.IO.Path]::Combine(
      [System.IO.Path]::GetDirectoryName($_.Path),
      "Mabinogi.exe"
    )
    $path -eq ${quotePowerShellLiteral(configuredPath)} -or
      $directoryNames -contains $parentDirectoryName -or
      (Test-Path -LiteralPath $mabinogiLauncherPath -PathType Leaf)
  }
} | Select-Object -First 1
[bool]$found
`
  const { stdout } = await execFileAsync(
    "powershell.exe",
    ["-NoProfile", "-NonInteractive", "-Command", script],
    { windowsHide: true, timeout: 15000 },
  )
  return stdout.trim().toLowerCase() === "true"
}

async function runMemoryHelper() {
  const statusPath = argumentValue("status-path")
  const controlPath = argumentValue("control-path")
  const purgeOnStart = argumentValue("purge-on-start") === "true"
  if (!statusPath || !controlPath) throw new Error("메모리 helper 제어 경로가 없습니다")
  const helperLock = await acquireHelperLock(statusPath)

  let stopping = false
  let gameActive = false
  let failure = null
  const memory = createMemoryManager({
    config,
    applyChanges: true,
    isGameActive: () => gameActive,
    isStopping: () => stopping,
  })

  const writeStatus = async values => {
    const memoryStatus = memory.getStatus()
    return writeJsonAtomic(statusPath, {
      ...memoryStatus,
      running: !stopping,
      gameActive,
      helperPid: process.pid,
      updatedAt: Date.now(),
      error: failure ?? memoryStatus.error,
      ...values,
    })
  }

  const writeStatusSafely = async values => {
    try {
      await writeStatus(values)
      return true
    } catch (error) {
      console.error("메모리 상태 기록 실패, 다음 주기에 다시 시도합니다", error)
      return false
    }
  }

  const requestStop = () => {
    stopping = true
  }
  process.on("SIGINT", requestStop)
  process.on("SIGTERM", requestStop)

  try {
    if (purgeOnStart) memory.purgeNow()
    await writeStatusSafely()
    while (!stopping) {
      const control = await readJson(controlPath)
      if (control?.command === "stop") break
      gameActive = await detectMabinogi()
      memory.checkNow()
      await writeStatusSafely()
      await delay(config.memoryCleaner?.pollIntervalMs ?? 1000)
    }
  } catch (error) {
    failure = serializeError(error)
    throw error
  } finally {
    stopping = true
    memory.stop()
    await writeStatusSafely({
      running: false,
      gameActive: false,
      stoppedAt: Date.now(),
    })
    await helperLock.release()
  }
}

async function runOptimizationHelper(operation) {
  const outputArgument = process.argv.find(argument => argument.startsWith("--output="))
  if (!outputArgument) throw new Error("관리자 작업 결과 경로가 없습니다")
  const outputPath = outputArgument.slice("--output=".length)

  try {
    const result = await operation()
    await writeFile(outputPath, JSON.stringify({ ok: true, data: result }), "utf8")
  } catch (error) {
    await writeFile(
      outputPath,
      JSON.stringify({ ok: false, error: serializeError(error) }),
      "utf8",
    )
    process.exitCode = 1
  }
}

const networkHelperMode = process.argv.includes("--network-helper")
const affinityHelperMode = process.argv.includes("--affinity-helper")
const memoryHelperMode = process.argv.includes("--memory-helper")

if (affinityHelperMode) {
  try {
    await runAffinityHelper()
  } catch (error) {
    console.error(error)
    process.exitCode = 1
  }
  app.exit(process.exitCode ?? 0)
} else if (memoryHelperMode) {
  try {
    await runMemoryHelper()
  } catch (error) {
    console.error(error)
    process.exitCode = 1
  }
  app.exit(process.exitCode ?? 0)
} else if (networkHelperMode) {
  await runOptimizationHelper(optimizeNetworkDirect)
  app.exit(process.exitCode ?? 0)
} else {
  if (!(await isAdministrator())) {
    if (await focusRunningPrimaryInstance(!startupTrayLaunch)) {
      app.exit(0)
    } else {
      await relaunchAsAdministrator()
      app.exit(0)
    }
  } else {
    const primaryInstance = app.requestSingleInstanceLock()
    if (!primaryInstance) {
      if (startupTrayLaunch) {
        app.exit(0)
      } else {
        void requestPrimaryWindowFocus()
          .catch(error => console.error("기존 창 포커스 요청 실패", error))
          .finally(() => app.exit(0))
      }
    } else {
      app.on("second-instance", focusPrimaryWindow)
      void startApplication().catch(error => {
        console.error(error)
        dialog.showErrorBox(
          "마비노기 렘 부스터 시작 실패",
          error?.message ?? String(error),
        )
        app.exit(1)
      })
    }
  }
}

async function checkGraphics() {
  const { checkGraphics: checkGraphicsStatus } = await import("../src/graphics.mjs")
  return checkGraphicsStatus(await resolveMabinogiExecutablePath())
}

async function optimizeGraphics() {
  const { applyGraphicsGoals } = await import("../src/graphics.mjs")
  return applyGraphicsGoals(await resolveMabinogiExecutablePath())
}

async function checkNetwork() {
  const [fastPing, tcpAutoTuning] = await Promise.all([
    ensureFastPingForPrimaryInterface(),
    ensureTcpAutoTuningNormal(),
  ])
  return {
    fastPing,
    tcpAutoTuning,
    optimized: (fastPing.supported === false || fastPing.configured)
      && tcpAutoTuning.optimized,
  }
}

const memoryStatusReader = createMemoryManager({
  config,
  applyChanges: false,
  isGameActive: () => false,
  isStopping: () => false,
})
let memoryStartPromise = null
let frameBoostStartupPromise = null
let frameBoostDesiredEnabled = true

function getMemoryPaths() {
  const base = join(app.getPath("userData"), "memory")
  return {
    statusPath: join(base, "status.json"),
    controlPath: join(base, "control.json"),
  }
}

async function readMemoryRuntimeStatus() {
  const { statusPath } = getMemoryPaths()
  const status = await readRuntimeStatusJson(statusPath)
  const fresh = status?.updatedAt
    && Date.now() - status.updatedAt < Math.max(
      (config.memoryCleaner?.pollIntervalMs ?? 1000) * 4,
      15000,
    )
  return {
    ...(status ?? {}),
    running: Boolean(status?.running && fresh),
    gameActive: Boolean(status?.gameActive && status?.running && fresh),
  }
}

async function checkMemory() {
  const runtime = await readMemoryRuntimeStatus()
  if (runtime.running) return runtime
  return {
    ...memoryStatusReader.getStatus(),
    running: false,
    gameActive: false,
    error: runtime.error ?? null,
  }
}

let affinityStartPromise = null
let cpuReorderPromise = null
const NIC_STATUS_CACHE_MS = 30000
let nicStatusCache = null
let nicStatusCheckedAt = 0
let nicStatusPromise = null

function setNicStatusCache(status) {
  nicStatusCache = status
  nicStatusCheckedAt = Date.now()
  return status
}

async function getCachedNicStatus({ refresh = false } = {}) {
  if (
    !refresh
    && nicStatusCache
    && Date.now() - nicStatusCheckedAt < NIC_STATUS_CACHE_MS
  ) {
    return nicStatusCache
  }
  nicStatusPromise ??= getNicRssAffinityStatus()
    .catch(error => ({
      supported: false,
      optimized: false,
      reason: error?.message ?? String(error),
    }))
    .then(setNicStatusCache)
    .finally(() => {
      nicStatusPromise = null
    })
  return nicStatusPromise
}

function invalidateNicStatusCache() {
  nicStatusCheckedAt = 0
}

function getAffinityPaths() {
  const base = join(app.getPath("userData"), "affinity")
  return {
    statusPath: join(base, "status.json"),
    controlPath: join(base, "control.json"),
    statePath: join(base, "runtime-state.json"),
    appliedMarkerPath: join(base, "applied-marker.json"),
  }
}

function getMabinogiPathStatePath() {
  return join(app.getPath("userData"), "game", "path.json")
}

function isMabinogiExecutablePath(value) {
  if (typeof value !== "string" || !value.trim()) return false
  const normalized = value.replaceAll("/", "\\")
  const executableName = String(config.gameExecutableName ?? "Client.exe")
  return normalized.split("\\").at(-1)?.toLowerCase() === executableName.toLowerCase()
    && existsSync(normalized)
}

function adoptMabinogiExecutablePath(value) {
  if (!isMabinogiExecutablePath(value)) return false
  if (value.toLowerCase() === activeMabinogiExecutablePath.toLowerCase()) return false
  activeMabinogiExecutablePath = value
  return true
}

async function refreshPathDependentStatuses() {
  const [dxvkResult, graphicsResult] = await Promise.allSettled([
    refreshDxvkRuntimeStatus({ force: true }),
    checkGraphics(),
  ])
  if (dxvkResult.status === "rejected") {
    console.error("마비노기 경로 변경 후 DXVK 재확인 실패", dxvkResult.reason)
  }
  if (graphicsResult.status === "rejected") {
    console.error("마비노기 경로 변경 후 그래픽 설정 재확인 실패", graphicsResult.reason)
  } else if (
    primaryWindow
    && !primaryWindow.isDestroyed()
    && !primaryWindow.webContents.isDestroyed()
  ) {
    primaryWindow.webContents.send(
      "optimization:graphics-status-changed",
      graphicsResult.value,
    )
  }
}

function observeMabinogiExecutablePath(value) {
  if (!adoptMabinogiExecutablePath(value)) return false
  void refreshPathDependentStatuses()
  return true
}

async function loadMabinogiExecutablePath() {
  const state = await readJson(getMabinogiPathStatePath())
  adoptMabinogiExecutablePath(state?.executablePath)
  return activeMabinogiExecutablePath
}

function startMabinogiPathStateMonitor() {
  clearInterval(gamePathStateMonitor)
  gamePathStateMonitor = setInterval(async () => {
    if (gamePathStateReading) return
    gamePathStateReading = true
    try {
      const state = await readJson(getMabinogiPathStatePath())
      observeMabinogiExecutablePath(state?.executablePath)
    } catch (error) {
      console.error("마비노기 실행 경로 상태 확인 실패", error)
    } finally {
      gamePathStateReading = false
    }
  }, 500)
}

async function resolveMabinogiExecutablePath() {
  if (isMabinogiExecutablePath(activeMabinogiExecutablePath)) {
    return activeMabinogiExecutablePath
  }
  await loadMabinogiExecutablePath()
  if (isMabinogiExecutablePath(activeMabinogiExecutablePath)) {
    return activeMabinogiExecutablePath
  }
  const { statusPath } = getAffinityPaths()
  const status = await readRuntimeStatusJson(statusPath)
  if (isMabinogiExecutablePath(status?.gameExecutablePath)) {
    activeMabinogiExecutablePath = status.gameExecutablePath
    return activeMabinogiExecutablePath
  }
  return config.gameExecutable
}

async function readAffinityRuntimeStatus() {
  const { statusPath } = getAffinityPaths()
  const status = await readRuntimeStatusJson(statusPath)
  const fresh = status?.updatedAt
    && Date.now() - status.updatedAt < Math.max(config.pollIntervalMs * 4, 30000)
  const characterSimplification = fresh && status?.characterSimplification
    ? status.characterSimplification
    : await getCharacterSimplificationStatus()
  const logicalCpuCount = cpus().length
  const half = logicalCpuCount / 2
  const detectedGameExecutablePath = isMabinogiExecutablePath(status?.gameExecutablePath)
    ? status.gameExecutablePath
    : null
  if (detectedGameExecutablePath) observeMabinogiExecutablePath(detectedGameExecutablePath)
  return {
    running: Boolean(status?.running && fresh),
    gameActive: Boolean(status?.gameActive && fresh),
    gameExecutablePath: detectedGameExecutablePath ?? activeMabinogiExecutablePath,
    includeNic: Boolean(status?.includeNic),
    nicManaged: Boolean(status?.nicManaged),
    backgroundCpuRange: status?.backgroundCpuRange ?? `0-${half - 1}`,
    gameCpuRange: status?.gameCpuRange ?? `${half}-${logicalCpuCount - 1}`,
    cpuReorder: fresh ? status?.cpuReorder ?? null : null,
    error: status?.error ?? null,
    nicStatus: status?.nicStatus ?? null,
    renderer: fresh && status?.renderer
      ? status.renderer
      : { mode: "not-running", version: null },
    characterSimplification,
    dxvk: dxvkRuntimeStatus,
    conflictingPrograms: Array.isArray(status?.conflictingPrograms)
      ? status.conflictingPrograms
      : [],
  }
}

async function checkAffinity({ refreshNic = false } = {}) {
  const runtime = await readAffinityRuntimeStatus()
  if (!refreshNic && runtime.nicStatus) setNicStatusCache(runtime.nicStatus)
  const nic = await getCachedNicStatus({ refresh: refreshNic })
  const previousNicFailureResolved = runtime.running === false
    && nic.optimized
    && runtime.error?.message?.startsWith("NIC RSS affinity")
  return {
    ...runtime,
    error: previousNicFailureResolved ? null : runtime.error,
    nic,
  }
}

function quotePowerShellLiteral(value) {
  return `'${String(value).replaceAll("'", "''")}'`
}

function quoteProcessArgument(value) {
  return quotePowerShellLiteral(`"${String(value).replaceAll('"', '\\"')}"`)
}

async function isAdministrator() {
  const script = [
    "$identity = [Security.Principal.WindowsIdentity]::GetCurrent()",
    "$principal = [Security.Principal.WindowsPrincipal]::new($identity)",
    "$principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)",
  ].join("\n")
  const { stdout } = await execFileAsync(
    "powershell.exe",
    ["-NoProfile", "-NonInteractive", "-Command", script],
    { windowsHide: true, timeout: 15000 },
  )
  return stdout.trim().toLowerCase() === "true"
}

async function relaunchAsAdministrator() {
  const relaunchArguments = app.isPackaged
    ? process.argv.slice(1)
    : [root, ...process.argv.slice(2)]
  const workingDirectory = app.isPackaged ? dirname(process.execPath) : root
  if (!relaunchArguments.includes("--elevated-relaunch")) {
    relaunchArguments.push("--elevated-relaunch")
  }
  const argumentList = relaunchArguments.map(quoteProcessArgument).join(", ")
  const script = [
    "$ErrorActionPreference = 'Stop'",
    `Start-Process -FilePath ${quotePowerShellLiteral(process.execPath)} -ArgumentList @(${argumentList}) -Verb RunAs -WorkingDirectory ${quotePowerShellLiteral(workingDirectory)}`,
  ].join("\n")
  const encodedCommand = Buffer.from(script, "utf16le").toString("base64")

  try {
    await execFileAsync(
      "powershell.exe",
      ["-NoProfile", "-NonInteractive", "-EncodedCommand", encodedCommand],
      { windowsHide: true, timeout: 120000 },
    )
  } catch {
    throw new Error("관리자 권한이 필요합니다. UAC 요청을 승인한 뒤 다시 실행하세요")
  }
}

async function getStartupTraySetting() {
  if (!app.isPackaged) {
    return {
      supported: false,
      enabled: false,
      reason: "설치된 앱에서만 사용할 수 있습니다",
    }
  }
  const script = [
    `$task = Get-ScheduledTask -TaskName ${quotePowerShellLiteral(startupTrayTaskName)} -ErrorAction SilentlyContinue`,
    "if ($null -eq $task) {",
    "  [pscustomobject]@{ exists = $false; enabled = $false; matches = $false } | ConvertTo-Json -Compress",
    "  exit 0",
    "}",
    "$action = @($task.Actions)[0]",
    `[pscustomobject]@{ exists = $true; enabled = $task.State -ne 'Disabled'; matches = ($action.Execute -ieq ${quotePowerShellLiteral(process.execPath)} -and $action.Arguments -eq '--startup-tray') } | ConvertTo-Json -Compress`,
  ].join("\n")
  const { stdout } = await execFileAsync(
    "powershell.exe",
    ["-NoProfile", "-NonInteractive", "-Command", script],
    { windowsHide: true, timeout: 15000 },
  )
  const state = JSON.parse(stdout.trim())
  return {
    supported: true,
    enabled: Boolean(state.exists && state.enabled && state.matches),
    reason: state.exists && !state.matches ? "등록된 실행 경로가 현재 설치 위치와 다릅니다" : null,
  }
}

async function setStartupTraySetting(enabled) {
  if (!app.isPackaged) throw new Error("설치된 앱에서만 사용할 수 있습니다")
  const script = enabled
    ? [
        "$ErrorActionPreference = 'Stop'",
        "$userId = [Security.Principal.WindowsIdentity]::GetCurrent().Name",
        `$action = New-ScheduledTaskAction -Execute ${quotePowerShellLiteral(process.execPath)} -Argument '--startup-tray'`,
        "$trigger = New-ScheduledTaskTrigger -AtLogOn -User $userId",
        "$principal = New-ScheduledTaskPrincipal -UserId $userId -LogonType Interactive -RunLevel Highest",
        "$settings = New-ScheduledTaskSettingsSet -MultipleInstances IgnoreNew -ExecutionTimeLimit ([TimeSpan]::Zero) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries",
        `Register-ScheduledTask -TaskName ${quotePowerShellLiteral(startupTrayTaskName)} -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Force | Out-Null`,
      ].join("\n")
    : [
        "$ErrorActionPreference = 'Stop'",
        `$task = Get-ScheduledTask -TaskName ${quotePowerShellLiteral(startupTrayTaskName)} -ErrorAction SilentlyContinue`,
        "if ($null -ne $task) {",
        `  Unregister-ScheduledTask -TaskName ${quotePowerShellLiteral(startupTrayTaskName)} -Confirm:$false`,
        "}",
      ].join("\n")
  await execFileAsync(
    "powershell.exe",
    ["-NoProfile", "-NonInteractive", "-Command", script],
    { windowsHide: true, timeout: 30000 },
  )
  const state = await getStartupTraySetting()
  if (state.enabled !== Boolean(enabled)) {
    throw new Error("Windows 시작 프로그램 설정을 확인하지 못했습니다")
  }
  return state
}

async function getTurboKeySetting() {
  const paths = getTurboKeyPaths()
  const [settings, status, installation] = await Promise.all([
    readJson(paths.settingsPath),
    readRuntimeStatusJson(paths.statusPath),
    getCachedTurboKeyInstallation(paths.directory),
  ])
  const statusFresh = Date.now() - Number(status?.updatedAt ?? 0) < 5000
  const enabled = Boolean(settings?.enabled && installation.installed)
  const intervalMs = normalizeTurboKeyIntervalMs(settings?.intervalMs)
  const running = Boolean(
    turboKeyProcess
    && turboKeyProcess.exitCode === null
    && status?.running
    && statusFresh,
  )
  return {
    installed: installation.installed,
    updateRequired: Boolean(installation.updateRequired),
    helperVersion: turboKeyHelperVersion,
    enabled,
    running,
    keys: normalizeTurboKeyCodes(settings?.keys),
    intervalMs,
    repeatHz: Math.round(1000 / intervalMs),
    gameOnly: true,
    reason: installation.reason
      ?? (statusFresh ? status?.error : null)
      ?? (enabled && !running ? "터보 키 프로세스가 실행 중이 아닙니다" : null),
  }
}

async function getCachedTurboKeyInstallation(directory, { refresh = false } = {}) {
  if (refresh || !turboKeyInstallationCache) {
    turboKeyInstallationCache = await getTurboKeyHelperInstallation(directory)
  }
  return turboKeyInstallationCache
}

async function updateTurboKeyHelperIfNeeded(installation) {
  if (!installation?.updateRequired || !installation.acceptedAt) return installation
  const paths = getTurboKeyPaths()
  turboKeyInstallationCache = await installTurboKeyHelper({
    directory: paths.directory,
    appVersion: app.getVersion(),
    acceptedAt: installation.acceptedAt,
    localSourcePath: app.isPackaged
      ? null
      : join(root, "native", "turbo-key", "bin", "turbo-key-helper.exe"),
  })
  return turboKeyInstallationCache
}

async function waitForTurboKeyStatus(predicate, timeoutMs = 3000) {
  const deadline = Date.now() + timeoutMs
  const { statusPath } = getTurboKeyPaths()
  while (Date.now() < deadline) {
    const status = await readRuntimeStatusJson(statusPath)
    if (predicate(status)) return status
    await delay(50)
  }
  return null
}

function waitForTurboKeyProcessExit(child, timeoutMs) {
  if (!child || child.exitCode !== null) return Promise.resolve(true)
  return new Promise(resolve => {
    const timer = setTimeout(() => {
      child.removeListener("exit", onExit)
      resolve(false)
    }, timeoutMs)
    const onExit = () => {
      clearTimeout(timer)
      resolve(true)
    }
    child.once("exit", onExit)
  })
}

async function launchTurboKeyHelper(
  keys = defaultTurboKeyCodes,
  intervalMs = normalizeTurboKeyIntervalMs(),
) {
  if (turboKeyProcess && turboKeyProcess.exitCode === null) {
    return getTurboKeySetting()
  }
  const installation = await getCachedTurboKeyInstallation(
    getTurboKeyPaths().directory,
    { refresh: true },
  )
  if (!installation.installed) {
    throw new Error(installation.reason ?? "터보 키를 먼저 다운로드하세요")
  }
  const executablePath = installation.executablePath
  const paths = getTurboKeyPaths()
  await mkdir(dirname(paths.statusPath), { recursive: true })
  await Promise.all([
    unlink(paths.statusPath).catch(() => {}),
    unlink(paths.controlPath).catch(() => {}),
  ])
  const allocation = resolveCpuAllocation(cpus().length)
  const latencyMask = allocation.alternateGameMask || allocation.backgroundMask
  const child = spawn(executablePath, [
    `--status-path=${paths.statusPath}`,
    `--control-path=${paths.controlPath}`,
    `--parent-pid=${process.pid}`,
    `--affinity-mask=0x${latencyMask.toString(16)}`,
    `--keys=${normalizeTurboKeyCodes(keys).join(",")}`,
    `--interval-ms=${normalizeTurboKeyIntervalMs(intervalMs)}`,
  ], {
    windowsHide: true,
    stdio: "ignore",
  })
  turboKeyProcess = child
  let spawnError = null
  child.once("error", error => {
    spawnError = error
  })
  child.once("exit", () => {
    if (turboKeyProcess === child) turboKeyProcess = null
  })
  const status = await waitForTurboKeyStatus(
    value => value?.running || value?.error || spawnError || child.exitCode !== null,
  )
  if (!status?.running) {
    try {
      if (child.pid && child.exitCode === null) child.kill()
    } catch {
    }
    await waitForTurboKeyProcessExit(child, 1000)
    if (turboKeyProcess === child) turboKeyProcess = null
    throw new Error(
      status?.error
      ?? spawnError?.message
      ?? "터보 키 실행을 확인하지 못했습니다",
    )
  }
  return getTurboKeySetting()
}

async function stopTurboKeyHelper() {
  const paths = getTurboKeyPaths()
  const child = turboKeyProcess
  if (!child || child.exitCode !== null) {
    turboKeyProcess = null
    return getTurboKeySetting()
  }
  await writeJsonAtomic(paths.controlPath, {
    command: "stop",
    requestedAt: Date.now(),
  })
  await waitForTurboKeyStatus(value => value?.running === false, 2000)
  let exited = await waitForTurboKeyProcessExit(child, 1000)
  if (!exited) {
    child.kill()
    exited = await waitForTurboKeyProcessExit(child, 500)
  }
  if (!exited) throw new Error("터보 키 프로세스를 종료하지 못했습니다")
  if (turboKeyProcess === child) turboKeyProcess = null
  return getTurboKeySetting()
}

async function setTurboKeySetting(setting) {
  const paths = getTurboKeyPaths()
  const enabled = Boolean(setting?.enabled)
  const keys = normalizeTurboKeyCodes(setting?.keys)
  const intervalMs = normalizeTurboKeyIntervalMs(setting?.intervalMs)
  if (enabled) {
    await stopTurboKeyHelper()
    await launchTurboKeyHelper(keys, intervalMs)
    try {
      await writeJsonAtomic(paths.settingsPath, {
        enabled: true,
        keys,
        intervalMs,
        updatedAt: Date.now(),
      })
    } catch (error) {
      await stopTurboKeyHelper().catch(() => {})
      throw error
    }
  } else {
    await stopTurboKeyHelper()
    await writeJsonAtomic(paths.settingsPath, {
      enabled: false,
      keys,
      intervalMs,
      updatedAt: Date.now(),
    })
  }
  return getTurboKeySetting()
}

async function downloadTurboKeyHelper() {
  await stopTurboKeyHelper()
  const paths = getTurboKeyPaths()
  turboKeyInstallationCache = await installTurboKeyHelper({
    directory: paths.directory,
    appVersion: app.getVersion(),
    acceptedAt: Date.now(),
    localSourcePath: app.isPackaged
      ? null
      : join(root, "native", "turbo-key", "bin", "turbo-key-helper.exe"),
  })
  const settings = await readJson(paths.settingsPath)
  await writeJsonAtomic(paths.settingsPath, {
    enabled: false,
    keys: normalizeTurboKeyCodes(settings?.keys),
    intervalMs: normalizeTurboKeyIntervalMs(settings?.intervalMs),
    updatedAt: Date.now(),
  })
  return getTurboKeySetting()
}

async function uninstallTurboKeyHelper() {
  await stopTurboKeyHelper()
  const paths = getTurboKeyPaths()
  const settings = await readJson(paths.settingsPath)
  await writeJsonAtomic(paths.settingsPath, {
    enabled: false,
    keys: normalizeTurboKeyCodes(settings?.keys),
    intervalMs: normalizeTurboKeyIntervalMs(settings?.intervalMs),
    updatedAt: Date.now(),
  })
  turboKeyInstallationCache = await removeTurboKeyHelper(paths.directory)
  return getTurboKeySetting()
}

async function ensureTurboKeyStarted() {
  const settings = await readJson(getTurboKeyPaths().settingsPath)
  let installation = await getCachedTurboKeyInstallation(
    getTurboKeyPaths().directory,
    { refresh: true },
  )
  installation = await updateTurboKeyHelperIfNeeded(installation)
  if (!settings?.enabled) return
  if (!installation.installed) return
  await launchTurboKeyHelper(
    normalizeTurboKeyCodes(settings.keys),
    normalizeTurboKeyIntervalMs(settings.intervalMs),
  )
}

async function launchMemoryHelper({ purgeOnStart = false } = {}) {
  const paths = getMemoryPaths()
  await mkdir(dirname(paths.statusPath), { recursive: true })
  await Promise.all([
    unlink(paths.statusPath).catch(() => {}),
    unlink(paths.controlPath).catch(() => {}),
  ])

  const helperArguments = [
    "--memory-helper",
    `--status-path=${paths.statusPath}`,
    `--control-path=${paths.controlPath}`,
    `--purge-on-start=${purgeOnStart}`,
  ]
  if (!app.isPackaged) helperArguments.unshift(root)
  const argumentList = helperArguments.map(quoteProcessArgument).join(", ")
  const script = [
    "$ErrorActionPreference = 'Stop'",
    `$process = Start-Process -FilePath ${quotePowerShellLiteral(process.execPath)} -ArgumentList @(${argumentList}) -WindowStyle Hidden -PassThru`,
    "$process.Id",
  ].join("\n")
  const encodedCommand = Buffer.from(script, "utf16le").toString("base64")

  try {
    await execFileAsync(
      "powershell.exe",
      ["-NoProfile", "-NonInteractive", "-EncodedCommand", encodedCommand],
      { windowsHide: true, timeout: 120000 },
    )
  } catch {
    throw new Error("메모리 최적화 helper 실행에 실패했습니다")
  }

  for (let attempt = 0; attempt < 30; attempt++) {
    const status = await readRuntimeStatusJson(paths.statusPath)
    if (status?.error) throw new Error(status.error.message ?? "메모리 최적화 시작 실패")
    if (status?.running) return checkMemory()
    await delay(500)
  }
  await writeJsonAtomic(paths.controlPath, {
    command: "stop",
    requestedAt: Date.now(),
    reason: "start-timeout",
  })
  throw new Error("메모리 최적화 helper가 제한 시간 안에 시작되지 않았습니다")
}

async function stopMemoryHelper() {
  if (memoryStartPromise) await memoryStartPromise.catch(() => {})
  const paths = getMemoryPaths()
  const current = await checkMemory()
  if (!current.running) return current

  await writeJsonAtomic(paths.controlPath, { command: "stop", requestedAt: Date.now() })
  for (let attempt = 0; attempt < 30; attempt++) {
    const status = await readRuntimeStatusJson(paths.statusPath)
    if (status?.running === false) return checkMemory()
    await delay(500)
  }
  throw new Error("메모리 최적화 helper가 제한 시간 안에 종료되지 않았습니다")
}

async function setMemoryEnabled(enabled, { purgeOnStart = false } = {}) {
  if (typeof enabled !== "boolean") throw new Error("메모리 최적화 활성화 여부가 올바르지 않습니다")
  const current = await checkMemory()
  if (enabled) {
    if (current.running) return current
    memoryStartPromise ??= launchMemoryHelper({ purgeOnStart })
    try {
      return await memoryStartPromise
    } finally {
      memoryStartPromise = null
    }
  }
  return stopMemoryHelper()
}

async function launchAffinityHelper(includeNic, inheritedNicManaged = false) {
  const paths = getAffinityPaths()
  const gamePathStatePath = getMabinogiPathStatePath()
  await Promise.all([
    mkdir(dirname(paths.statusPath), { recursive: true }),
    mkdir(dirname(gamePathStatePath), { recursive: true }),
  ])
  await Promise.all([
    unlink(paths.statusPath).catch(() => {}),
    unlink(paths.controlPath).catch(() => {}),
  ])

  const helperArguments = [
    "--affinity-helper",
    `--status-path=${paths.statusPath}`,
    `--control-path=${paths.controlPath}`,
    `--affinity-state-path=${paths.statePath}`,
    `--applied-marker-path=${paths.appliedMarkerPath}`,
    `--game-path-state-path=${gamePathStatePath}`,
    `--include-nic=${includeNic}`,
    `--nic-managed=${inheritedNicManaged}`,
  ]
  if (!app.isPackaged) helperArguments.unshift(root)

  try {
    if (app.isPackaged) {
      const argumentList = helperArguments.map(quoteProcessArgument).join(", ")
      const script = [
        "$ErrorActionPreference = 'Stop'",
        `$process = Start-Process -FilePath ${quotePowerShellLiteral(process.execPath)} -ArgumentList @(${argumentList}) -WindowStyle Hidden -WorkingDirectory ${quotePowerShellLiteral(dirname(process.execPath))} -PassThru`,
        "$process.Id",
      ].join("\n")
      const encodedCommand = Buffer.from(script, "utf16le").toString("base64")
      await execFileAsync(
        "powershell.exe",
        ["-NoProfile", "-NonInteractive", "-EncodedCommand", encodedCommand],
        { windowsHide: true, timeout: 120000 },
      )
    } else {
      const child = spawn(process.execPath, helperArguments, {
        cwd: root,
        detached: true,
        stdio: "ignore",
        windowsHide: true,
      })
      await new Promise((resolve, reject) => {
        child.once("spawn", resolve)
        child.once("error", reject)
      })
      child.unref()
    }
  } catch {
    throw new Error("Affinity helper 실행에 실패했습니다")
  }

  for (let attempt = 0; attempt < 450; attempt++) {
    const status = await readRuntimeStatusJson(paths.statusPath)
    if (status?.error) throw new Error(status.error.message ?? "Affinity 최적화 시작 실패")
    if (status?.running) {
      if (status.nicStatus) setNicStatusCache(status.nicStatus)
      return readAffinityRuntimeStatus()
    }
    await delay(100)
  }
  await writeJsonAtomic(paths.controlPath, {
    command: "stop",
    requestedAt: Date.now(),
    reason: "start-timeout",
  })
  throw new Error("Affinity helper가 제한 시간 안에 시작되지 않았습니다")
}

async function stopAffinityHelper({ reset = false } = {}) {
  if (affinityStartPromise) await affinityStartPromise.catch(() => {})
  const paths = getAffinityPaths()
  const current = await readAffinityRuntimeStatus()
  if (!current.running) {
    if (reset) {
      await launchAffinityHelper(false, current.nicManaged)
      return stopAffinityHelper({ reset: true })
    }
    return checkAffinity()
  }

  await writeJsonAtomic(paths.controlPath, {
    command: reset ? "reset" : "stop",
    requestedAt: Date.now(),
  })
  for (let attempt = 0; attempt < 450; attempt++) {
    const status = await readRuntimeStatusJson(paths.statusPath)
    if (status?.running === false) {
      if (status.nicStatus) setNicStatusCache(status.nicStatus)
      else if (current.nicManaged) invalidateNicStatusCache()
      return checkAffinity()
    }
    await delay(100)
  }
  throw new Error("Affinity helper가 제한 시간 안에 종료되지 않았습니다")
}

async function setAffinityEnabled({ enabled, includeNic = false } = {}) {
  if (typeof enabled !== "boolean") throw new Error("Affinity 활성화 여부가 올바르지 않습니다")
  const current = await readAffinityRuntimeStatus()
  if (enabled) {
    if (current.running) return checkAffinity()
    affinityStartPromise ??= launchAffinityHelper(Boolean(includeNic), current.nicManaged)
    try {
      await affinityStartPromise
      return checkAffinity()
    } finally {
      affinityStartPromise = null
    }
  }
  return stopAffinityHelper()
}

async function resetAllAffinities() {
  frameBoostDesiredEnabled = false
  updateApplicationTrayIcon()
  return stopAffinityHelper({ reset: true })
}

async function setFrameBoostEnabled({ enabled, includeNic = false } = {}) {
  if (typeof enabled !== "boolean") throw new Error("프레임 부스트 활성화 여부가 올바르지 않습니다")
  const previousDesiredEnabled = frameBoostDesiredEnabled
  frameBoostDesiredEnabled = enabled
  updateApplicationTrayIcon()
  try {
    const [affinity, memory] = await Promise.all([
      setAffinityEnabled({ enabled, includeNic }),
      setMemoryEnabled(enabled),
    ])
    return { affinity, memory }
  } catch (error) {
    frameBoostDesiredEnabled = previousDesiredEnabled
    updateApplicationTrayIcon()
    throw error
  }
}

async function runCpuReorder() {
  if (cpuReorderPromise) return cpuReorderPromise
  cpuReorderPromise = (async () => {
    if (applicationExitInProgress || closeRequestPending) {
      throw new Error("앱 종료 처리 중에는 CPU 재정렬을 실행할 수 없습니다")
    }
    if (!frameBoostDesiredEnabled) {
      throw new Error("실시간 부스트가 켜져 있을 때만 CPU 재정렬을 실행할 수 있습니다")
    }

    const [affinity, memory] = await Promise.all([
      readAffinityRuntimeStatus(),
      readMemoryRuntimeStatus(),
    ])
    if (!(affinity.running && memory.running)) {
      throw new Error("실시간 부스트가 켜져 있을 때만 CPU 재정렬을 실행할 수 있습니다")
    }
    if (!affinity.gameActive) throw new Error("실행 중인 마비노기를 찾을 수 없습니다")
    if (affinity.cpuReorder?.state === "running") throw new Error("CPU 재정렬이 이미 실행 중입니다")

    const requestId = randomUUID()
    const { controlPath, statusPath } = getAffinityPaths()
    await writeJsonAtomic(controlPath, {
      command: "cpu-reorder",
      requestId,
      requestedAt: Date.now(),
    })

    for (let attempt = 0; attempt < 100; attempt++) {
      const status = await readRuntimeStatusJson(statusPath)
      if (status?.running === false) throw new Error("Affinity helper가 CPU 재정렬 중 종료되었습니다")
      if (status?.cpuReorder?.requestId === requestId) {
        if (status.cpuReorder.state === "completed") return readAffinityRuntimeStatus()
        if (status.cpuReorder.state === "failed") {
          throw new Error(status.cpuReorder.error?.message ?? "CPU 재정렬에 실패했습니다")
        }
      }
      await delay(100)
    }
    throw new Error("CPU 재정렬이 제한 시간 안에 완료되지 않았습니다")
  })()

  try {
    return await cpuReorderPromise
  } finally {
    cpuReorderPromise = null
  }
}

async function resetFrameBoost() {
  const [affinity, memory] = await Promise.all([
    resetAllAffinities(),
    setMemoryEnabled(false),
  ])
  return { affinity, memory }
}

async function ensureAffinityStarted() {
  const current = await readAffinityRuntimeStatus()
  if (current.running) return current
  affinityStartPromise ??= launchAffinityHelper(false, current.nicManaged)
  try {
    return await affinityStartPromise
  } finally {
    affinityStartPromise = null
  }
}

async function ensureFrameBoostStarted() {
  return Promise.all([
    ensureAffinityStarted(),
    setMemoryEnabled(true, { purgeOnStart: true }),
  ])
}

async function requestApplicationExitConfirmation({ nativeDialog = false } = {}) {
  if (applicationExitInProgress || (closeRequestPending && !nativeDialog)) return
  if (!primaryWindow || primaryWindow.isDestroyed() || primaryWindow.webContents.isDestroyed()) {
    await finishApplicationExit("keep")
    return
  }
  closeRequestPending = true
  try {
    const paths = getAffinityPaths()
    const [affinityState, affinityStatus, appliedMarker] = await Promise.all([
      readJson(paths.statePath),
      readRuntimeStatusJson(paths.statusPath),
      readJson(paths.appliedMarkerPath),
    ])
    const { hasLiveAppliedAffinityEntries } = await import("../src/affinity.mjs")
    const hasLiveAppliedAffinity = await hasLiveAppliedAffinityEntries([
      ...(affinityState?.entries ?? []),
      ...(appliedMarker?.entries ?? []),
    ])
    const hasManagedNic = Boolean(
      affinityStatus?.nicManaged
      || appliedMarker?.nicManaged,
    )
    const exitConfirmation = assessExitConfirmation({
      hasLiveAppliedAffinity,
      hasManagedNic,
    })
    if (!exitConfirmation.confirm) {
      if (exitConfirmation.removeAppliedMarker) {
        await unlink(paths.appliedMarkerPath).catch(() => {})
      }
      await finishApplicationExitWithoutAppliedBoost()
      return
    }
  } catch (error) {
    console.error("종료 전 부스트 적용 기록 확인 실패", error)
  }
  if (nativeDialog) {
    const result = await dialog.showMessageBox({
      type: "question",
      title: "프로그램 종료",
      message: "프레임 부스트를 정지할까요?",
      detail: "부스트를 유지한 채로 종료하면 마비노기 외 프로그램들의 성능이 제한될 수 있습니다.",
      buttons: [
        "부스트 설정 되돌린 후 종료",
        "적용 유지 후 종료",
        "취소",
      ],
      defaultId: 0,
      cancelId: 2,
      noLink: true,
    })
    if (result.response === 2) {
      closeRequestPending = false
      return
    }
    await finishApplicationExit(result.response === 0 ? "reset" : "keep")
    return
  }
  primaryWindow.webContents.send("application:close-requested")
}

async function finishApplicationExitWithoutAppliedBoost() {
  if (applicationExitInProgress) return { closing: true }
  applicationExitInProgress = true
  closeRequestPending = false
  const requestedAt = Date.now()
  const results = await Promise.allSettled([
    writeJsonAtomic(getAffinityPaths().controlPath, {
      command: "stop",
      requestedAt,
      reason: "application-exit-without-applied-boost",
    }),
    writeJsonAtomic(getMemoryPaths().controlPath, {
      command: "stop",
      requestedAt,
      reason: "application-exit-without-applied-boost",
    }),
    stopTurboKeyHelper(),
  ])
  for (const result of results) {
    if (result.status === "rejected") console.error(result.reason)
  }
  app.quit()
  return { closing: true }
}

async function finishApplicationExit(action) {
  if (action === "cancel") {
    closeRequestPending = false
    return { closing: false }
  }
  if (action !== "reset" && action !== "keep") {
    throw new Error("종료 방식이 올바르지 않습니다")
  }
  if (applicationExitInProgress) return { closing: true }

  applicationExitInProgress = true
  closeRequestPending = false
  const results = await Promise.allSettled([
    action === "reset" ? resetAllAffinities() : stopAffinityHelper(),
    stopMemoryHelper(),
    stopTurboKeyHelper(),
  ])
  for (const result of results) {
    if (result.status === "rejected") console.error(result.reason)
  }
  app.quit()
  return { closing: true }
}

async function runElevatedOptimization(helperFlag, failureMessage) {
  const outputPath = join(tmpdir(), `nogirem-${randomUUID()}.json`)
  const helperArguments = app.isPackaged
    ? [helperFlag, `--output=${outputPath}`]
    : [root, helperFlag, `--output=${outputPath}`]
  const argumentList = helperArguments.map(quoteProcessArgument).join(", ")
  const script = [
    "$ErrorActionPreference = 'Stop'",
    `$process = Start-Process -FilePath ${quotePowerShellLiteral(process.execPath)} -ArgumentList @(${argumentList}) -Wait -PassThru`,
    "exit $process.ExitCode",
  ].join("\n")
  const encodedCommand = Buffer.from(script, "utf16le").toString("base64")

  try {
    await execFileAsync(
      "powershell.exe",
      ["-NoProfile", "-NonInteractive", "-EncodedCommand", encodedCommand],
      { windowsHide: true, timeout: 120000 },
    )
  } catch (error) {
    try {
      const payload = JSON.parse(await readFile(outputPath, "utf8"))
      if (!payload.ok) throw new Error(payload.error?.message ?? failureMessage)
    } catch (resultError) {
      if (resultError.code === "ENOENT") {
        throw new Error("관리자 helper 실행에 실패했습니다")
      }
      throw resultError
    }
  }

  try {
    const payload = JSON.parse(await readFile(outputPath, "utf8"))
    if (!payload.ok) throw new Error(payload.error?.message ?? failureMessage)
    return payload.data
  } finally {
    await unlink(outputPath).catch(() => {})
  }
}

async function optimizeNetworkDirect() {
  const tcpAutoTuning = await ensureTcpAutoTuningNormal({ applyChanges: true })
  const fastPing = await ensureFastPingForPrimaryInterface({
    applyChanges: true,
    restartAfterApply: true,
  })
  return {
    fastPing,
    tcpAutoTuning,
    optimized: (fastPing.supported === false || fastPing.configured)
      && tcpAutoTuning.optimized,
  }
}

async function optimizeNetwork() {
  return runElevatedOptimization("--network-helper", "네트워크 최적화 실패")
}

function getDxvkDirectory() {
  return join(app.getPath("appData"), "마비노기 렘 부스터", "vulkan")
}

async function getDxvkTargetPath() {
  return join(dirname(await resolveMabinogiExecutablePath()), "d3d9_dxvk.dll")
}

function getDxvkLatestCachePath() {
  return join(getDxvkDirectory(), "latest.json")
}

function getDxvkReleasesCachePath() {
  return join(getDxvkDirectory(), "releases.json")
}

function isValidCachedDxvkRelease(release) {
  return Boolean(
    release
    && /^v\d+(?:\.\d+){1,3}$/.test(release.version ?? "")
    && /^[a-f0-9]{64}$/i.test(release.archiveSha256 ?? "")
    && /^dxvk-\d+(?:\.\d+){1,3}\.tar\.gz$/i.test(release.archiveName ?? "")
    && String(release.downloadUrl ?? "").startsWith(
      "https://github.com/doitsujin/dxvk/releases/download/",
    )
  )
}

async function loadCachedDxvkReleases() {
  const cache = await readJson(getDxvkReleasesCachePath())
  const checkedAt = Date.parse(cache?.checkedAt ?? "")
  const releases = Array.isArray(cache?.releases)
    ? cache.releases.filter(isValidCachedDxvkRelease)
    : []
  if (!Number.isFinite(checkedAt)) return []
  dxvkReleasesCache = releases
  dxvkReleasesCheckedAt = checkedAt
  dxvkReleasesCacheError = typeof cache?.error === "string" ? cache.error : null
  return releases
}

async function getCachedDxvkReleases() {
  const cacheFresh = Date.now() - dxvkReleasesCheckedAt < dxvkReleaseCacheDurationMs
  if (cacheFresh) {
    if (dxvkReleasesCache.length > 0) return dxvkReleasesCache
    throw new Error(dxvkReleasesCacheError ?? "DXVK 릴리즈 목록을 확인할 수 없습니다")
  }
  if (dxvkReleasesCheckPromise) return dxvkReleasesCheckPromise
  dxvkReleasesCheckPromise = (async () => {
    try {
      const releases = await getDxvkReleases()
      dxvkReleasesCache = releases
      dxvkReleasesCheckedAt = Date.now()
      dxvkReleasesCacheError = null
      await writeJsonAtomic(getDxvkReleasesCachePath(), {
        checkedAt: new Date(dxvkReleasesCheckedAt).toISOString(),
        releases,
        error: null,
      }).catch(() => {})
      return releases
    } catch (error) {
      dxvkReleasesCheckedAt = Date.now()
      dxvkReleasesCacheError = error?.message ?? String(error)
      await writeJsonAtomic(getDxvkReleasesCachePath(), {
        checkedAt: new Date(dxvkReleasesCheckedAt).toISOString(),
        releases: dxvkReleasesCache,
        error: dxvkReleasesCacheError,
      }).catch(() => {})
      if (dxvkReleasesCache.length > 0) return dxvkReleasesCache
      throw error
    } finally {
      dxvkReleasesCheckPromise = null
    }
  })()
  return dxvkReleasesCheckPromise
}

async function evaluateDxvkRuntimeStatus(latest) {
  const installed = await getInstalledDxvk(getDxvkDirectory())
  const deployment = await getDxvkDeploymentStatus(installed, await getDxvkTargetPath())
  const latestApplied = Boolean(
    installed.installed
    && installed.integrity
    && installed.current?.version === latest.version
    && installed.current?.archiveSha256 === latest.archiveSha256
    && deployment.matchesCurrent
  )
  return {
    state: latestApplied ? "latest" : "update-required",
    latestVersion: latest.version,
    error: null,
  }
}

async function loadCachedDxvkRuntimeStatus() {
  const latest = await readJson(getDxvkLatestCachePath())
  if (
    !latest?.version
    || !/^v\d+(?:\.\d+){1,3}$/.test(latest.version)
    || !/^[a-f0-9]{64}$/i.test(latest.archiveSha256 ?? "")
  ) return dxvkRuntimeStatus
  dxvkRuntimeStatus = await evaluateDxvkRuntimeStatus(latest)
  return dxvkRuntimeStatus
}

async function refreshDxvkRuntimeStatus({ force = false } = {}) {
  if (dxvkRuntimeCheckPromise) {
    if (!force) return dxvkRuntimeCheckPromise
    await dxvkRuntimeCheckPromise
  }
  const previousStatus = dxvkRuntimeStatus
  if (!["latest", "update-required"].includes(previousStatus.state)) {
    dxvkRuntimeStatus = {
      state: "checking",
      latestVersion: previousStatus.latestVersion,
      error: null,
    }
  }
  dxvkRuntimeCheckPromise = (async () => {
    try {
      const [latest] = await getCachedDxvkReleases()
      if (!latest) throw new Error("DXVK 최신 정식 릴리즈를 찾지 못했습니다")
      dxvkRuntimeStatus = await evaluateDxvkRuntimeStatus(latest)
      await writeJsonAtomic(getDxvkLatestCachePath(), {
        version: latest.version,
        archiveSha256: latest.archiveSha256,
        checkedAt: new Date().toISOString(),
      }).catch(() => {})
    } catch (error) {
      dxvkRuntimeStatus = ["latest", "update-required"].includes(previousStatus.state)
        ? { ...previousStatus, error: serializeError(error) }
        : {
            state: "unavailable",
            latestVersion: null,
            error: serializeError(error),
          }
    } finally {
      dxvkRuntimeCheckPromise = null
    }
    return dxvkRuntimeStatus
  })()
  return dxvkRuntimeCheckPromise
}

function scheduleDxvkRuntimeRefresh() {
  clearTimeout(dxvkRuntimeRefreshTimer)
  dxvkRuntimeRefreshTimer = setTimeout(() => {
    void refreshDxvkRuntimeStatus({ force: true })
      .finally(scheduleDxvkRuntimeRefresh)
  }, dxvkReleaseCacheDurationMs)
}

async function getDxvkManagerStatus({ checkLatest = false } = {}) {
  const installed = await getInstalledDxvk(getDxvkDirectory())
  const deployment = await getDxvkDeploymentStatus(installed, await getDxvkTargetPath())
  const releases = checkLatest ? await getCachedDxvkReleases() : []
  const latest = releases[0] ?? null
  return {
    installed,
    deployment,
    latest,
    releases,
    updateAvailable: latest
      ? !installed.installed
        || !installed.integrity
        || !deployment.matchesCurrent
        || installed.current?.version !== latest.version
        || installed.current?.archiveSha256 !== latest.archiveSha256
      : null,
    storagePath: getDxvkDirectory(),
  }
}

async function updateDxvk(version) {
  if (await detectMabinogi().catch(() => false)) {
    throw new Error("마비노기가 실행 중일 때에는 DXVK를 교체할 수 없습니다")
  }
  dxvkUpdatePromise ??= (async () => {
    const releases = await getCachedDxvkReleases()
    const result = await installDxvkVersion(
      getDxvkDirectory(),
      version,
      globalThis.fetch,
      releases,
    )
    const deployment = await applyInstalledDxvk(
      getDxvkDirectory(),
      await getDxvkTargetPath(),
    )
    return { ...result, deployment }
  })()
  try {
    const result = await dxvkUpdatePromise
    await refreshDxvkRuntimeStatus({ force: true })
    return {
      ...result,
      storagePath: getDxvkDirectory(),
    }
  } catch (error) {
    if (await detectMabinogi().catch(() => false)) {
      throw new Error("마비노기가 실행 중일 때에는 DXVK를 교체할 수 없습니다")
    }
    throw error
  } finally {
    dxvkUpdatePromise = null
  }
}

function resultOf(promise) {
  return promise.then(
    data => ({ ok: true, data }),
    error => ({ ok: false, error: serializeError(error) }),
  )
}

function registerIpc() {
  ipcMain.handle("optimization:get-status", async () => {
    await frameBoostStartupPromise
    return Promise.all([
      resultOf(checkGraphics()),
      resultOf(checkNetwork()),
      resultOf(checkAffinity({ refreshNic: true })),
      resultOf(checkMemory()),
    ]).then(([graphics, network, affinity, memory]) => ({
      graphics,
      nvidia: graphics,
      network,
      affinity,
      memory,
    }))
  })
  ipcMain.handle("optimization:refresh-graphics", () => checkGraphics())
  ipcMain.handle("optimization:refresh-nvidia", () => checkGraphics())
  ipcMain.handle("optimization:refresh-network", () => checkNetwork())
  ipcMain.handle("optimization:refresh-affinity", () => checkAffinity({ refreshNic: true }))
  ipcMain.handle("optimization:get-affinity-runtime", async () => {
    const runtime = await readAffinityRuntimeStatus()
    if (
      frameBoostDesiredEnabled
      && !runtime.running
      && !applicationExitInProgress
      && !closeRequestPending
    ) {
      return ensureAffinityStarted()
    }
    return runtime
  })
  ipcMain.handle("optimization:refresh-memory", () => checkMemory())
  ipcMain.handle("optimization:get-memory-runtime", async () => {
    const runtime = await readMemoryRuntimeStatus()
    if (
      frameBoostDesiredEnabled
      && !runtime.running
      && !applicationExitInProgress
      && !closeRequestPending
    ) {
      return setMemoryEnabled(true)
    }
    return runtime
  })
  ipcMain.handle("optimization:optimize-graphics", () => optimizeGraphics())
  ipcMain.handle("optimization:optimize-nvidia", () => optimizeGraphics())
  ipcMain.handle("optimization:optimize-network", () => optimizeNetwork())
  ipcMain.handle("optimization:set-affinity-enabled", (_event, options) => {
    return setAffinityEnabled(options)
  })
  ipcMain.handle("optimization:reset-affinity", () => resetAllAffinities())
  ipcMain.handle("optimization:set-frame-boost-enabled", (_event, options) => {
    return setFrameBoostEnabled(options)
  })
  ipcMain.handle("optimization:run-cpu-reorder", () => runCpuReorder())
  ipcMain.handle("optimization:reset-frame-boost", () => resetFrameBoost())
  ipcMain.handle("optimization:set-memory-enabled", (_event, enabled) => {
    return setMemoryEnabled(enabled)
  })
  ipcMain.handle("application:begin-startup-reveal", event => {
    if (
      !startupTrayLaunch
      && BrowserWindow.fromWebContents(event.sender) === primaryWindow
    ) {
      beginPrimaryWindowReveal()
    }
  })
  ipcMain.handle("application:get-launch-context", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) {
      throw new Error("허용되지 않은 실행 상태 요청입니다")
    }
    return { startupTray: startupTrayLaunch || primaryRendererRecoveryMode }
  })
  ipcMain.handle("application:get-startup-tray-setting", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) {
      throw new Error("허용되지 않은 시작 설정 요청입니다")
    }
    return getStartupTraySetting()
  })
  ipcMain.handle("application:set-startup-tray-setting", (event, enabled) => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) {
      throw new Error("허용되지 않은 시작 설정 변경 요청입니다")
    }
    if (typeof enabled !== "boolean") throw new Error("시작 설정 값이 올바르지 않습니다")
    return setStartupTraySetting(enabled)
  })
  ipcMain.handle("application:get-turbo-key-setting", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) {
      throw new Error("허용되지 않은 터보 키 설정 요청입니다")
    }
    return getTurboKeySetting()
  })
  ipcMain.handle("application:download-turbo-key-helper", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) {
      throw new Error("허용되지 않은 터보 키 다운로드 요청입니다")
    }
    return downloadTurboKeyHelper()
  })
  ipcMain.handle("application:remove-turbo-key-helper", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) {
      throw new Error("허용되지 않은 터보 키 제거 요청입니다")
    }
    return uninstallTurboKeyHelper()
  })
  ipcMain.handle("application:set-turbo-key-setting", (event, setting) => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) {
      throw new Error("허용되지 않은 터보 키 설정 변경 요청입니다")
    }
    if (
      !setting
      || typeof setting !== "object"
      || typeof setting.enabled !== "boolean"
      || !Array.isArray(setting.keys)
      || !Number.isInteger(setting.intervalMs)
      || normalizeTurboKeyIntervalMs(setting.intervalMs) !== setting.intervalMs
    ) {
      throw new Error("터보 키 설정 값이 올바르지 않습니다")
    }
    return setTurboKeySetting(setting)
  })
  ipcMain.handle("application:get-visual-activity", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) return false
    return isPrimaryWindowVisuallyActive()
  })
  ipcMain.handle("application:get-update-state", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) {
      throw new Error("허용되지 않은 업데이트 상태 요청입니다")
    }
    return applicationUpdateState
  })
  ipcMain.handle("application:check-update", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) {
      throw new Error("허용되지 않은 업데이트 확인 요청입니다")
    }
    return checkForApplicationUpdate()
  })
  ipcMain.handle("application:install-update", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) {
      throw new Error("허용되지 않은 업데이트 설치 요청입니다")
    }
    return installDownloadedApplicationUpdate()
  })
  ipcMain.handle("application:request-close", () => requestApplicationExitConfirmation())
  ipcMain.handle("application:minimize-to-tray", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) {
      throw new Error("허용되지 않은 최소화 요청입니다")
    }
    return minimizePrimaryWindowToTray()
  })
  ipcMain.handle("application:open-character-guide", () => {
    openCharacterSimplificationGuide()
  })
  ipcMain.handle("application:open-dxvk-manager", () => {
    openDxvkManager()
  })
  ipcMain.handle("application:open-dxvk-guide", () => {
    openDxvkGuide()
  })
  ipcMain.handle("application:open-creator-channel", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) return false
    return shell.openExternal(creatorChannelUrl).then(() => true)
  })
  ipcMain.handle("application:get-creator-channel", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) {
      throw new Error("허용되지 않은 채널 정보 요청입니다")
    }
    return readCreatorChannelProfile()
  })
  ipcMain.handle("application:get-creator-prompt-dismissed", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) return true
    return readCreatorPromptDismissed()
  })
  ipcMain.handle("application:dismiss-creator-prompt", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) return false
    return dismissCreatorPrompt()
  })
  ipcMain.handle("application:open-direct-donation", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) return false
    return shell.openExternal(directDonationUrl).then(() => true)
  })
  ipcMain.handle("application:open-operation-policy", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== primaryWindow) return false
    return shell.openExternal(operationPolicyUrl).then(() => true)
  })
  ipcMain.handle("dxvk-guide:request-close", event => {
    const window = BrowserWindow.fromWebContents(event.sender)
    if (window !== dxvkGuideWindow) {
      throw new Error("허용되지 않은 DXVK 가이드 닫기 요청입니다")
    }
    window.close()
  })
  ipcMain.handle("dxvk:request-close", event => {
    const window = BrowserWindow.fromWebContents(event.sender)
    if (window !== dxvkManagerWindow) {
      throw new Error("허용되지 않은 DXVK 창 닫기 요청입니다")
    }
    window.close()
  })
  ipcMain.handle("dxvk:get-status", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== dxvkManagerWindow) {
      throw new Error("허용되지 않은 DXVK 상태 요청입니다")
    }
    return getDxvkManagerStatus()
  })
  ipcMain.handle("dxvk:check-update", event => {
    if (BrowserWindow.fromWebContents(event.sender) !== dxvkManagerWindow) {
      throw new Error("허용되지 않은 DXVK 업데이트 확인 요청입니다")
    }
    return getDxvkManagerStatus({ checkLatest: true })
  })
  ipcMain.handle("dxvk:install-update", (event, version) => {
    if (BrowserWindow.fromWebContents(event.sender) !== dxvkManagerWindow) {
      throw new Error("허용되지 않은 DXVK 설치 요청입니다")
    }
    return updateDxvk(version)
  })
  ipcMain.on("character-guide:drag-start", (event, point) => {
    if (
      !characterGuideWindow
      || characterGuideWindow.isDestroyed()
      || event.sender !== characterGuideWindow.webContents
      || !Number.isFinite(point?.screenX)
      || !Number.isFinite(point?.screenY)
    ) return
    const [windowX, windowY] = characterGuideWindow.getPosition()
    characterGuideDrag = {
      window: characterGuideWindow,
      pointerX: point.screenX,
      pointerY: point.screenY,
      windowX,
      windowY,
    }
  })
  ipcMain.on("character-guide:drag-move", (event, point) => {
    const drag = characterGuideDrag
    if (
      !drag
      || drag.window.isDestroyed()
      || event.sender !== drag.window.webContents
      || !Number.isFinite(point?.screenX)
      || !Number.isFinite(point?.screenY)
    ) return
    drag.window.setPosition(
      Math.round(drag.windowX + point.screenX - drag.pointerX),
      Math.round(drag.windowY + point.screenY - drag.pointerY),
    )
  })
  ipcMain.on("character-guide:drag-end", event => {
    if (characterGuideDrag?.window.webContents === event.sender) {
      characterGuideDrag = null
    }
  })
  ipcMain.on("dxvk-guide:drag-start", (event, point) => {
    if (
      !dxvkGuideWindow
      || dxvkGuideWindow.isDestroyed()
      || event.sender !== dxvkGuideWindow.webContents
      || !Number.isFinite(point?.screenX)
      || !Number.isFinite(point?.screenY)
    ) return
    const [windowX, windowY] = dxvkGuideWindow.getPosition()
    dxvkGuideDrag = {
      window: dxvkGuideWindow,
      pointerX: point.screenX,
      pointerY: point.screenY,
      windowX,
      windowY,
    }
  })
  ipcMain.on("dxvk-guide:drag-move", (event, point) => {
    const drag = dxvkGuideDrag
    if (
      !drag
      || drag.window.isDestroyed()
      || event.sender !== drag.window.webContents
      || !Number.isFinite(point?.screenX)
      || !Number.isFinite(point?.screenY)
    ) return
    drag.window.setPosition(
      Math.round(drag.windowX + point.screenX - drag.pointerX),
      Math.round(drag.windowY + point.screenY - drag.pointerY),
    )
  })
  ipcMain.on("dxvk-guide:drag-end", event => {
    if (dxvkGuideDrag?.window.webContents === event.sender) {
      dxvkGuideDrag = null
    }
  })
  ipcMain.handle("application:confirm-close", (_event, action) => {
    return finishApplicationExit(action)
  })
}

function disableProductionRefresh(window) {
  if (!app.isPackaged) return
  window.webContents.on("before-input-event", (event, input) => {
    const key = input.key.toLowerCase()
    if (key === "f5" || ((input.control || input.meta) && key === "r")) {
      event.preventDefault()
    }
  })
}

function openDxvkManager() {
  if (dxvkManagerWindow && !dxvkManagerWindow.isDestroyed()) {
    if (dxvkManagerWindow.isMinimized()) dxvkManagerWindow.restore()
    dxvkManagerWindow.setIgnoreMouseEvents(false)
    dxvkManagerWindow.show()
    dxvkManagerWindow.moveTop()
    dxvkManagerWindow.focus()
    return
  }

  const window = new BrowserWindow({
    width: 560,
    height: 430,
    show: false,
    opacity: 0,
    resizable: false,
    maximizable: false,
    parent: primaryWindow ?? undefined,
    title: "Vulkan 업데이트 관리",
    icon: iconPath,
    autoHideMenuBar: true,
    skipTaskbar: true,
    frame: false,
    roundedCorners: true,
    backgroundColor: "#101214",
    webPreferences: {
      preload: dxvkManagerPreloadPath,
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  })
  disableProductionRefresh(window)
  dxvkManagerWindow = window
  observeInternalWindowVisualActivity(window)
  window.webContents.setWindowOpenHandler(({ url }) => {
    if (url === "https://github.com/doitsujin/dxvk") {
      void shell.openExternal(url)
    }
    return { action: "deny" }
  })
  let opacityTimer = null
  let closing = false
  const animateOpacity = (from, to, duration, onComplete) => {
    clearInterval(opacityTimer)
    const startedAt = Date.now()
    opacityTimer = setInterval(() => {
      if (window.isDestroyed()) {
        clearInterval(opacityTimer)
        opacityTimer = null
        return
      }
      const progress = Math.min(1, (Date.now() - startedAt) / duration)
      window.setOpacity(from + (to - from) * progress)
      if (progress < 1) return
      clearInterval(opacityTimer)
      opacityTimer = null
      onComplete?.()
    }, 16)
  }
  window.once("ready-to-show", () => {
    if (window.isDestroyed()) return
    window.center()
    window.show()
    window.focus()
    animateOpacity(0, 1, 300)
  })
  window.on("close", event => {
    if (closing) return
    event.preventDefault()
    closing = true
    animateOpacity(window.getOpacity(), 0, 300, () => window.destroy())
  })
  window.on("closed", () => {
    clearInterval(opacityTimer)
    if (dxvkManagerWindow === window) dxvkManagerWindow = null
    if (!applicationExitInProgress) focusPrimaryWindow()
  })
  const builtManagerPath = join(root, "dist", "dxvk-manager.html")
  const loading = process.argv.includes("--dev")
    ? window.loadURL("http://localhost:5173/dxvk-manager.html")
    : window.loadFile(existsSync(builtManagerPath)
      ? builtManagerPath
      : join(root, "dxvk-manager.html"))
  void loading
    .catch(error => showWindowLoadError(window, error))
    .catch(error => console.error("DXVK 관리 화면 로드 실패", error))
}

function openDxvkGuide() {
  if (dxvkGuideWindow && !dxvkGuideWindow.isDestroyed()) {
    if (dxvkGuideWindow.isMinimized()) dxvkGuideWindow.restore()
    dxvkGuideWindow.setIgnoreMouseEvents(false)
    dxvkGuideWindow.setAlwaysOnTop(true, "screen-saver", 1)
    dxvkGuideWindow.show()
    dxvkGuideWindow.moveTop()
    dxvkGuideWindow.focus()
    return
  }

  const window = new BrowserWindow({
    width: 920,
    height: 900,
    show: false,
    opacity: 0,
    resizable: false,
    maximizable: false,
    parent: primaryWindow ?? undefined,
    title: "DXVK Vulkan 설정 안내",
    icon: iconPath,
    autoHideMenuBar: true,
    alwaysOnTop: true,
    skipTaskbar: true,
    frame: false,
    transparent: true,
    roundedCorners: true,
    backgroundColor: "#00000000",
    webPreferences: {
      preload: dxvkGuidePreloadPath,
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  })
  disableProductionRefresh(window)
  dxvkGuideWindow = window
  observeInternalWindowVisualActivity(window)
  let opacityTimer = null
  let closing = false
  const animateOpacity = (from, to, duration, onComplete) => {
    clearInterval(opacityTimer)
    const startedAt = Date.now()
    opacityTimer = setInterval(() => {
      if (window.isDestroyed()) {
        clearInterval(opacityTimer)
        opacityTimer = null
        return
      }
      const progress = Math.min(1, (Date.now() - startedAt) / duration)
      window.setOpacity(from + (to - from) * progress)
      if (progress < 1) return
      clearInterval(opacityTimer)
      opacityTimer = null
      onComplete?.()
    }, 16)
  }
  window.setAlwaysOnTop(true, "screen-saver", 1)
  window.once("ready-to-show", () => {
    if (window.isDestroyed()) return
    window.center()
    window.show()
    window.focus()
    animateOpacity(0, 0.9, 300)
  })
  window.on("show", () => {
    if (!window.isDestroyed()) window.setAlwaysOnTop(true, "screen-saver", 1)
  })
  window.on("close", event => {
    if (closing) return
    event.preventDefault()
    closing = true
    animateOpacity(window.getOpacity(), 0, 300, () => window.destroy())
  })
  window.on("closed", () => {
    clearInterval(opacityTimer)
    if (dxvkGuideDrag?.window === window) dxvkGuideDrag = null
    if (dxvkGuideWindow === window) dxvkGuideWindow = null
    if (!applicationExitInProgress) focusPrimaryWindow()
  })
  const builtGuidePath = join(root, "dist", "dxvk-guide.html")
  const loading = process.argv.includes("--dev")
    ? window.loadURL("http://localhost:5173/dxvk-guide.html")
    : window.loadFile(existsSync(builtGuidePath)
      ? builtGuidePath
      : join(root, "dxvk-guide.html"))
  void loading
    .catch(error => showWindowLoadError(window, error))
    .catch(error => console.error("DXVK 가이드 화면 로드 실패", error))
}

function openCharacterSimplificationGuide() {
  if (characterGuideWindow && !characterGuideWindow.isDestroyed()) {
    if (characterGuideWindow.isMinimized()) characterGuideWindow.restore()
    characterGuideWindow.setIgnoreMouseEvents(false)
    characterGuideWindow.setAlwaysOnTop(true, "screen-saver", 1)
    characterGuideWindow.show()
    characterGuideWindow.moveTop()
    characterGuideWindow.focus()
    return
  }

  const window = new BrowserWindow({
    width: 920,
    height: 900,
    minWidth: 520,
    minHeight: 360,
    show: false,
    title: "주변 캐릭터 강제 간소화",
    icon: iconPath,
    autoHideMenuBar: true,
    alwaysOnTop: true,
    skipTaskbar: true,
    frame: false,
    transparent: true,
    opacity: 0,
    roundedCorners: true,
    backgroundColor: "#00000000",
    webPreferences: {
      preload: characterGuidePreloadPath,
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
    },
  })
  disableProductionRefresh(window)
  characterGuideWindow = window
  observeInternalWindowVisualActivity(window)
  let opacityTimer = null
  let closing = false
  const animateOpacity = (from, to, duration, onComplete) => {
    clearInterval(opacityTimer)
    const startedAt = Date.now()
    opacityTimer = setInterval(() => {
      if (window.isDestroyed()) {
        clearInterval(opacityTimer)
        opacityTimer = null
        return
      }
      const progress = Math.min(1, (Date.now() - startedAt) / duration)
      window.setOpacity(from + (to - from) * progress)
      if (progress < 1) return
      clearInterval(opacityTimer)
      opacityTimer = null
      onComplete?.()
    }, 16)
  }
  window.setAlwaysOnTop(true, "screen-saver", 1)
  window.once("ready-to-show", () => {
    if (window.isDestroyed()) return
    window.show()
    animateOpacity(0, 0.9, 300)
  })
  window.on("show", () => {
    if (!window.isDestroyed()) window.setAlwaysOnTop(true, "screen-saver", 1)
  })
  window.on("close", event => {
    if (closing) return
    event.preventDefault()
    closing = true
    animateOpacity(window.getOpacity(), 0, 300, () => window.destroy())
  })
  window.on("closed", () => {
    clearInterval(opacityTimer)
    if (characterGuideDrag?.window === window) characterGuideDrag = null
    if (characterGuideWindow === window) characterGuideWindow = null
    if (!applicationExitInProgress) focusPrimaryWindow()
  })
  const builtGuidePath = join(root, "dist", "character-guide.html")
  const loading = process.argv.includes("--dev")
    ? window.loadURL("http://localhost:5173/character-guide.html")
    : window.loadFile(existsSync(builtGuidePath)
      ? builtGuidePath
      : join(root, "character-guide.html"))
  void loading
    .catch(error => showWindowLoadError(window, error))
    .catch(error => console.error("간소화 안내 오류 화면 로드 실패", error))
}

async function installCharacterSimplificationFile() {
  const sourcePath = join(root, "assets", characterSimplificationFileName)
  const settingsDirectory = join(app.getPath("documents"), "마비노기", "설정")
  const destinationDirectory = join(settingsDirectory, "목록")
  await mkdir(destinationDirectory, { recursive: true })
  await copyFile(sourcePath, join(destinationDirectory, characterSimplificationFileName))
  await unlink(join(settingsDirectory, characterSimplificationFileName)).catch(error => {
    if (error?.code !== "ENOENT") throw error
  })
}

function escapeHtml(value) {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;")
}

async function showWindowLoadError(window, error) {
  console.error("렌더러 화면 로드 실패", error)
  const message = escapeHtml(error?.message ?? String(error))
  const html = `<!doctype html>
<html lang="ko">
<meta charset="UTF-8">
<meta name="color-scheme" content="dark">
<title>nogirem 화면 로드 실패</title>
<style>
body{margin:0;padding:48px;background:#101214;color:#f3f5f7;font:15px/1.6 "Segoe UI",sans-serif}
main{max-width:760px;margin:0 auto;padding:28px;border:1px solid #49302d;border-radius:12px;background:#211918}
h1{margin:0 0 14px;font-size:24px}
p{margin:0;color:#efaaa3;white-space:pre-wrap;overflow-wrap:anywhere}
</style>
<main><h1>화면을 불러오지 못했습니다</h1><p>${message}</p></main>
</html>`
  await window.loadURL(`data:text/html;charset=utf-8,${encodeURIComponent(html)}`)
  window.setOpacity(1)
  window.show()
}

function beginPrimaryWindowReveal() {
  if (primaryWindowRevealStarted || !primaryWindow || primaryWindow.isDestroyed()) return
  clearTimeout(primaryWindowRevealWatchdogTimer)
  primaryWindowRevealWatchdogTimer = null
  primaryWindowRevealStarted = true
  writeStartupLog("메인 창 표시 시작")
  const window = primaryWindow
  const startedAt = Date.now()
  window.setOpacity(0)
  window.show()
  focusPrimaryWindow()

  const revealFrame = () => {
    if (primaryWindow !== window || window.isDestroyed()) return
    const progress = Math.min(1, (Date.now() - startedAt) / 1000)
    window.setOpacity(progress)
    if (progress < 1) {
      primaryWindowRevealFrameTimer = setTimeout(revealFrame, 16)
    } else {
      primaryWindowRevealFrameTimer = null
    }
  }
  revealFrame()
}

function internalWindows() {
  return [
    characterGuideWindow,
    dxvkManagerWindow,
    dxvkGuideWindow,
  ].filter(window => window && !window.isDestroyed())
}

function hideInternalWindowsForTray() {
  for (const window of internalWindows()) {
    window.setIgnoreMouseEvents(true)
    window.setAlwaysOnTop(false)
    window.hide()
  }
}

function focusPrimaryWindow() {
  if (!primaryWindow || primaryWindow.isDestroyed()) {
    primaryWindowFocusPending = true
    return
  }
  primaryWindowFocusPending = false
  const restoringFromTray = primaryWindowSkippedFromTaskbar
  if (restoringFromTray) hideInternalWindowsForTray()
  if (primaryWindow.isMinimized()) primaryWindow.restore()
  primaryWindow.setFocusable(true)
  primaryWindow.setIgnoreMouseEvents(false)
  primaryWindow.setAlwaysOnTop(false)
  primaryWindow.setSkipTaskbar(false)
  primaryWindowSkippedFromTaskbar = false
  if (!primaryWindow.isVisible()) primaryWindow.show()
  const window = primaryWindow
  clearTimeout(primaryWindowFocusTimer)

  const applyFocus = allowRetry => {
    if (primaryWindow !== window || window.isDestroyed()) return
    window.focus()
    window.webContents.focus()
    if (allowRetry && !window.isFocused()) {
      primaryWindowFocusTimer = setTimeout(() => {
        primaryWindowFocusTimer = null
        applyFocus(false)
      }, primaryWindowFocusRetryDelayMs)
    }
  }
  primaryWindowFocusTimer = setTimeout(() => {
    primaryWindowFocusTimer = null
    applyFocus(true)
  }, 0)
}

function focusPrimaryWindowAfterTrayMenu() {
  clearTimeout(primaryWindowTrayRestoreTimer)
  primaryWindowTrayRestoreTimer = setTimeout(() => {
    primaryWindowTrayRestoreTimer = null
    focusPrimaryWindow()
  }, trayMenuCloseDelayMs)
}

async function requestApplicationExitFromTray() {
  await delay(trayMenuCloseDelayMs)
  const mainWindowVisible = Boolean(
    primaryWindow
    && !primaryWindow.isDestroyed()
    && primaryWindow.isVisible()
    && !primaryWindowSkippedFromTaskbar,
  )
  if (mainWindowVisible) focusPrimaryWindow()
  return requestApplicationExitConfirmation({ nativeDialog: !mainWindowVisible })
}

function ensureApplicationTray() {
  if (applicationTray && !applicationTray.isDestroyed()) return applicationTray
  applicationTray = new Tray(iconPath)
  updateApplicationTrayIcon()
  applicationTray.setToolTip("마비노기 렘 부스터")
  applicationTray.setContextMenu(Menu.buildFromTemplate([
    {
      label: "열기",
      click: focusPrimaryWindowAfterTrayMenu,
    },
    {
      label: "종료",
      click: () => {
        void requestApplicationExitFromTray()
          .catch(error => console.error("트레이 종료 요청 처리 실패", error))
      },
    },
  ]))
  applicationTray.on("click", focusPrimaryWindowAfterTrayMenu)
  applicationTray.on("double-click", focusPrimaryWindowAfterTrayMenu)
  return applicationTray
}

function updateApplicationTrayIcon() {
  if (!applicationTray || applicationTray.isDestroyed()) return
  applicationTray.setImage(frameBoostDesiredEnabled ? iconPath : pausedTrayIconPath)
}

function minimizePrimaryWindowToTray() {
  if (!primaryWindow || primaryWindow.isDestroyed()) return false
  clearTimeout(primaryWindowFocusTimer)
  clearTimeout(primaryWindowTrayRestoreTimer)
  primaryWindowFocusTimer = null
  primaryWindowTrayRestoreTimer = null
  ensureApplicationTray()
  hideInternalWindowsForTray()
  primaryWindow.setSkipTaskbar(true)
  primaryWindowSkippedFromTaskbar = true
  primaryWindow.hide()
  notifyPrimaryVisualActivity()
  return true
}

function isPrimaryWindowVisuallyActive() {
  const internalWindowFocused = [
    primaryWindow,
    characterGuideWindow,
    dxvkManagerWindow,
    dxvkGuideWindow,
  ].some(window => window && !window.isDestroyed() && window.isFocused())
  return Boolean(
    primaryWindow
    && !primaryWindow.isDestroyed()
    && primaryWindow.isVisible()
    && !primaryWindow.isMinimized()
    && internalWindowFocused
  )
}

function notifyPrimaryVisualActivity() {
  clearTimeout(primaryVisualActivityTimer)
  primaryVisualActivityTimer = setTimeout(() => {
    primaryVisualActivityTimer = null
    if (
      !primaryWindow
      || primaryWindow.isDestroyed()
      || primaryWindow.webContents.isDestroyed()
    ) return
    primaryWindow.webContents.send(
      "application:visual-activity-changed",
      isPrimaryWindowVisuallyActive(),
    )
  }, 50)
}

function observeInternalWindowVisualActivity(window) {
  for (const eventName of ["focus", "blur", "show", "hide", "minimize", "restore", "closed"]) {
    window.on(eventName, notifyPrimaryVisualActivity)
  }
}

function clearPrimaryRendererUnresponsiveTimer() {
  clearTimeout(primaryRendererUnresponsiveTimer)
  primaryRendererUnresponsiveTimer = null
}

function relaunchWithSandboxCompatibility(details) {
  if (
    !app.isPackaged
    || globalThis.__nogiremSandboxFallbackLaunch
    || sandboxCompatibilityRelaunching
    || details?.reason !== "launch-failed"
    || details?.exitCode !== 18
  ) return false

  sandboxCompatibilityRelaunching = true
  writeStartupLog("Chromium 샌드박스 호환 모드로 한 번 재실행")
  const args = process.argv.slice(1)
  if (!args.includes("--sandbox-fallback")) args.push("--sandbox-fallback")
  app.relaunch({ args })
  app.exit(0)
  return true
}

function recoverPrimaryRenderer(window, reason) {
  if (
    applicationExitInProgress
    || primaryWindow !== window
    || window.isDestroyed()
    || window.webContents.isDestroyed()
    || primaryRendererRecoveryInProgress
  ) return

  clearPrimaryRendererUnresponsiveTimer()
  clearTimeout(primaryRendererRecoveryResetTimer)
  primaryRendererRecoveryResetTimer = null
  primaryRendererRecoveryMode = true
  primaryRendererRecoveryInProgress = true
  console.error(`렌더러 자동 복구 시작: ${reason}`)

  window.webContents.once("did-finish-load", () => {
    if (primaryWindow !== window || window.isDestroyed()) return
    primaryRendererRecoveryInProgress = false
    if (primaryWindowSkippedFromTaskbar) window.hide()
    window.setSkipTaskbar(primaryWindowSkippedFromTaskbar)
    primaryRendererRecoveryResetTimer = setTimeout(() => {
      primaryRendererRecoveryMode = false
      primaryRendererRecoveryResetTimer = null
    }, 10_000)
  })
  window.webContents.reload()
}

function createWindow() {
  const window = new BrowserWindow({
    width: 640,
    height: 290,
    show: false,
    opacity: 0,
    icon: iconPath,
    resizable: false,
    maximizable: false,
    autoHideMenuBar: true,
    frame: false,
    roundedCorners: false,
    backgroundColor: "#ffffff",
    webPreferences: {
      preload: preloadPath,
      contextIsolation: true,
      nodeIntegration: false,
      sandbox: true,
      backgroundThrottling: false,
    },
  })
  disableProductionRefresh(window)
  window.webContents.on("preload-error", (_event, preload, error) => {
    console.error(`preload 로드 실패: ${preload}`, error)
  })
  window.webContents.on("did-fail-load", (_event, code, description, url, isMainFrame) => {
    if (isMainFrame) console.error(`렌더러 문서 로드 실패 (${code}): ${url}`, description)
  })
  window.webContents.on("console-message", (_event, details) => {
    if (details.level === "error") {
      console.error(`렌더러 오류: ${details.message} (${details.sourceId}:${details.lineNumber})`)
    }
  })
  window.webContents.on("render-process-gone", (_event, details) => {
    console.error("렌더러 프로세스 종료", details)
    if (relaunchWithSandboxCompatibility(details)) return
    recoverPrimaryRenderer(window, `프로세스 종료 (${details.reason})`)
  })
  window.on("unresponsive", () => {
    if (primaryRendererUnresponsiveTimer || primaryRendererRecoveryInProgress) return
    primaryRendererUnresponsiveTimer = setTimeout(() => {
      primaryRendererUnresponsiveTimer = null
      recoverPrimaryRenderer(window, "5초 이상 무응답")
    }, primaryRendererUnresponsiveTimeoutMs)
  })
  window.on("responsive", clearPrimaryRendererUnresponsiveTimer)
  primaryWindow = window
  primaryWindowSkippedFromTaskbar = false
  writeStartupLog("메인 창 생성 완료")
  if (!startupTrayLaunch) {
    clearTimeout(primaryWindowRevealWatchdogTimer)
    primaryWindowRevealWatchdogTimer = setTimeout(() => {
      primaryWindowRevealWatchdogTimer = null
      writeStartupLog("창 표시 watchdog 실행")
      beginPrimaryWindowReveal()
    }, primaryWindowRevealTimeoutMs)
  }
  for (const eventName of ["focus", "blur", "show", "hide", "minimize", "restore"]) {
    window.on(eventName, notifyPrimaryVisualActivity)
  }
  window.on("minimize", event => {
    event.preventDefault()
    minimizePrimaryWindowToTray()
  })
  window.on("close", event => {
    if (applicationExitInProgress) return
    event.preventDefault()
    void requestApplicationExitConfirmation()
      .catch(error => console.error("종료 요청 처리 실패", error))
  })
  window.on("closed", () => {
    clearPrimaryRendererUnresponsiveTimer()
    clearTimeout(primaryRendererRecoveryResetTimer)
    clearTimeout(primaryWindowFocusTimer)
    clearTimeout(primaryWindowTrayRestoreTimer)
    clearTimeout(primaryVisualActivityTimer)
    clearTimeout(primaryWindowRevealFrameTimer)
    clearTimeout(primaryWindowRevealWatchdogTimer)
    primaryWindowFocusTimer = null
    primaryWindowTrayRestoreTimer = null
    primaryVisualActivityTimer = null
    primaryWindowRevealFrameTimer = null
    primaryWindowRevealWatchdogTimer = null
    primaryRendererRecoveryResetTimer = null
    primaryRendererRecoveryMode = false
    primaryRendererRecoveryInProgress = false
    primaryWindowRevealStarted = false
    primaryWindowSkippedFromTaskbar = false
    if (primaryWindow === window) primaryWindow = null
  })

  const loading = process.argv.includes("--dev")
    ? window.loadURL("http://localhost:5173")
    : window.loadFile(join(root, "dist", "index.html"))
  void loading
    .then(() => {
      writeStartupLog("렌더러 문서 로드 완료")
      if (startupTrayLaunch) {
        window.setOpacity(1)
        window.setSkipTaskbar(true)
        primaryWindowSkippedFromTaskbar = true
      }
      if (primaryWindowFocusPending) focusPrimaryWindow()
    })
    .catch(error => showWindowLoadError(window, error))
    .catch(error => console.error("오류 화면 로드 실패", error))
}

async function startApplication() {
  writeStartupLog("Electron 초기화 시작")
  registerIpc()
  await app.whenReady()
  writeStartupLog("Electron 준비 완료")
  configureApplicationUpdater()
  if (startupTrayLaunch) ensureApplicationTray()
  createWindow()

  void startFocusRequestMonitor()
    .catch(error => console.error("포커스 요청 감시 시작 실패", error))
  startMabinogiPathStateMonitor()
  frameBoostStartupPromise = ensureFrameBoostStarted().catch(error => {
    console.error("앱 시작 프레임 부스트 자동 실행 실패", error)
  }).then(() => {
    writeStartupLog("프레임 부스트 초기화 처리 종료")
  })
  void ensureTurboKeyStarted().catch(error => {
    console.error("터보 키 자동 실행 실패", error)
  }).then(() => {
    writeStartupLog("터보 키 초기화 처리 종료")
  })
  void (async () => {
    await loadMabinogiExecutablePath()
      .catch(error => console.error("마비노기 경로 초기화 실패", error))
    writeStartupLog("마비노기 경로 초기화 처리 종료")
    await loadCachedDxvkReleases()
      .catch(error => console.error("DXVK 릴리스 캐시 로드 실패", error))
    await loadCachedDxvkRuntimeStatus().catch(error => {
      dxvkRuntimeStatus = {
        state: "checking",
        latestVersion: null,
        error: serializeError(error),
      }
    })
    void refreshDxvkRuntimeStatus().finally(scheduleDxvkRuntimeRefresh)
    await installCharacterSimplificationFile()
      .catch(error => console.error("주변 캐릭터 간소화 파일 설치 실패", error))
    writeStartupLog("백그라운드 초기화 완료")
  })()
  if (app.isPackaged) {
    applicationUpdateStartupTimer = setTimeout(() => {
      applicationUpdateStartupTimer = null
      void checkForApplicationUpdate()
    }, 3000)
    applicationUpdateCheckTimer = setInterval(() => {
      void checkForApplicationUpdate()
    }, 4 * 60 * 60 * 1000)
  }

  app.on("before-quit", event => {
    if (applicationExitInProgress) return
    event.preventDefault()
    void requestApplicationExitConfirmation()
      .catch(error => console.error("종료 요청 처리 실패", error))
  })

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) createWindow()
  })
  app.on("window-all-closed", () => app.quit())
  app.on("will-quit", () => {
    clearTimeout(applicationUpdateStartupTimer)
    applicationUpdateStartupTimer = null
    clearInterval(applicationUpdateCheckTimer)
    applicationUpdateCheckTimer = null
    clearApplicationUpdateCompletionTimer()
    clearApplicationUpdateStallTimer()
    clearPrimaryRendererUnresponsiveTimer()
    clearTimeout(primaryRendererRecoveryResetTimer)
    primaryRendererRecoveryResetTimer = null
    clearTimeout(primaryWindowRevealWatchdogTimer)
    primaryWindowRevealWatchdogTimer = null
    clearTimeout(dxvkRuntimeRefreshTimer)
    dxvkRuntimeRefreshTimer = null
    clearInterval(focusRequestMonitor)
    focusRequestMonitor = null
    clearInterval(gamePathStateMonitor)
    gamePathStateMonitor = null
    if (applicationTray && !applicationTray.isDestroyed()) applicationTray.destroy()
    applicationTray = null
    try {
      unlinkSync(primaryInstancePath)
    } catch (error) {
      if (error?.code !== "ENOENT") console.error("인스턴스 기록 삭제 실패", error)
    }
  })
}
