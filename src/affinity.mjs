import { execFile } from "node:child_process"
import { promisify } from "node:util"
import { cpus } from "node:os"
import { readFile, writeFile, unlink } from "node:fs/promises"
import { basename } from "node:path"
import { setTimeout as delay } from "node:timers/promises"
import { DataType, PointerType, createPointer, freePointer, load, open, restorePointer } from "ffi-rs"

const execFileAsync = promisify(execFile)

open({ library: "kernel32", path: "kernel32.dll" })

const PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
const PROCESS_SET_INFORMATION = 0x0200

function nativeCall(funcName, retType, paramsType, paramsValue) {
  return load({ library: "kernel32", funcName, retType, paramsType, paramsValue })
}

function openProcess(pid, writable = false) {
  return nativeCall(
    "OpenProcess",
    DataType.External,
    [DataType.U32, DataType.Boolean, DataType.U32],
    [PROCESS_QUERY_LIMITED_INFORMATION | (writable ? PROCESS_SET_INFORMATION : 0), false, pid],
  )
}

function lastError() {
  return Number(nativeCall("GetLastError", DataType.U32, [], []))
}

function closeHandle(handle) {
  if (handle) nativeCall("CloseHandle", DataType.Boolean, [DataType.External], [handle])
}

function readPointer(pointer, type) {
  const result = restorePointer({ paramsValue: pointer, retType: [type] })
  return Array.isArray(result) ? result[0] : result
}

function queryProcessPath(pid) {
  const handle = openProcess(pid)
  if (!handle) return null
  const capacity = 32768
  const buffer = Buffer.alloc(capacity * 2)
  const size = createPointer({ paramsType: [DataType.U32], paramsValue: [capacity] })
  try {
    const ok = nativeCall(
      "QueryFullProcessImageNameW",
      DataType.Boolean,
      [DataType.External, DataType.U32, DataType.U8Array, DataType.External],
      [handle, 0, buffer, size[0]],
    )
    if (!ok) return null
    const length = Number(readPointer(size, DataType.U32))
    return buffer.subarray(0, length * 2).toString("utf16le")
  } finally {
    freePointer({ paramsType: [DataType.U32], paramsValue: size, pointerType: PointerType.RsPointer })
    closeHandle(handle)
  }
}

function getAffinity(pid) {
  const handle = openProcess(pid)
  if (!handle) throw new Error("OpenProcess failed")
  const processMask = createPointer({ paramsType: [DataType.U64], paramsValue: [0] })
  const systemMask = createPointer({ paramsType: [DataType.U64], paramsValue: [0] })
  try {
    const ok = nativeCall(
      "GetProcessAffinityMask",
      DataType.Boolean,
      [DataType.External, DataType.External, DataType.External],
      [handle, processMask[0], systemMask[0]],
    )
    if (!ok) throw new Error("GetProcessAffinityMask failed")
    return BigInt(readPointer(processMask, DataType.U64))
  } finally {
    freePointer({ paramsType: [DataType.U64], paramsValue: processMask, pointerType: PointerType.RsPointer })
    freePointer({ paramsType: [DataType.U64], paramsValue: systemMask, pointerType: PointerType.RsPointer })
    closeHandle(handle)
  }
}

function setAffinity(pid, mask) {
  const handle = openProcess(pid, true)
  if (!handle) throw new Error(`OpenProcess failed (Win32 ${lastError()})`)
  try {
    const ok = nativeCall(
      "SetProcessAffinityMask",
      DataType.Boolean,
      [DataType.External, DataType.U64],
      [handle, Number(mask)],
    )
    if (!ok) throw new Error(`SetProcessAffinityMask failed (Win32 ${lastError()})`)
  } finally {
    closeHandle(handle)
  }
}

const normalizePath = value => value?.replaceAll("/", "\\").toLowerCase() ?? ""

export function buildCpuHalfMasks(logicalCpuCount) {
  if (logicalCpuCount < 4 || logicalCpuCount > 52 || logicalCpuCount % 2) {
    throw new Error(`Unsupported logical CPU count: ${logicalCpuCount}; expected an even count from 4 to 52.`)
  }
  const half = logicalCpuCount / 2
  const allMask = (1n << BigInt(logicalCpuCount)) - 1n
  const lowerHalfMask = (1n << BigInt(half)) - 1n
  return {
    half,
    allMask,
    lowerHalfMask,
    upperHalfMask: allMask ^ lowerHalfMask,
  }
}

export function pinCurrentProcessToBackgroundCpus() {
  const logicalCpuCount = cpus().length
  if (process.platform !== "win32" || process.arch !== "x64") {
    throw new Error("Windows x64 only.")
  }
  const { half: backgroundCpuCount, lowerHalfMask: backgroundMask } = buildCpuHalfMasks(logicalCpuCount)
  setAffinity(process.pid, backgroundMask)
  return {
    mask: `0x${backgroundMask.toString(16)}`,
    cpuRange: `0-${backgroundCpuCount - 1}`,
  }
}

export async function createAffinityManager({
  config,
  statePath,
  applyChanges,
  quiet,
  lastCoreMode,
  passiveMode,
  restoreOnGameExit = true,
}) {
  const logicalCpuCount = cpus().length

  if (process.platform !== "win32" || process.arch !== "x64") throw new Error("Windows x64 only.")
  const {
    half,
    allMask,
    lowerHalfMask,
    upperHalfMask,
  } = buildCpuHalfMasks(logicalCpuCount)
  const gameMask = passiveMode
    ? null
    : (lastCoreMode ? 3n << BigInt(logicalCpuCount - 2) : upperHalfMask)
  const backgroundMask = passiveMode ? upperHalfMask : allMask ^ gameMask
  const modeName = passiveMode ? "passive" : (lastCoreMode ? "last-core" : "half")
  const allocationDescription = passiveMode
    ? `game affinity unchanged, background CPUs ${half}-${logicalCpuCount - 1}`
    : (lastCoreMode
      ? `background CPUs 0-${logicalCpuCount - 3}, game CPUs ${logicalCpuCount - 2}-${logicalCpuCount - 1}`
      : `background CPUs 0-${half - 1}, game CPUs ${half}-${logicalCpuCount - 1}`)
  const configuredGamePath = normalizePath(config.gameExecutable)
  const gameExecutableName = String(
    config.gameExecutableName ?? basename(config.gameExecutable ?? "Client.exe"),
  ).toLowerCase()
  const gameDirectoryName = String(config.gameDirectoryName ?? "Mabinogi").toLowerCase()
  const excludeNames = new Set(config.excludeNames.map(value => value.toLowerCase()))
  const excludePatterns = config.excludeNamePatterns.map(value => new RegExp(value, "i"))
  const changed = new Map()
  const handled = new Set()
  let gameActive = false
  let latestGameStartTime = null
  let runningProcessNames = new Set()

  const isGame = processInfo => {
    const path = normalizePath(processInfo.path)
    if (!path || processInfo.name.toLowerCase() !== gameExecutableName) return false
    if (configuredGamePath && path === configuredGamePath) return true
    return path.split("\\").includes(gameDirectoryName)
  }

  async function listProcesses() {
    const ps = `$ErrorActionPreference='SilentlyContinue'; Get-Process | ForEach-Object { [pscustomobject]@{ pid=$_.Id; name=($_.ProcessName+'.exe'); path=$_.Path; startTime=if($_.StartTime){$_.StartTime.ToUniversalTime().ToString('O')}else{$null}; sessionId=$_.SessionId } } | ConvertTo-Json -Compress`
    const { stdout } = await execFileAsync(
      "powershell.exe",
      ["-NoProfile", "-NonInteractive", "-Command", ps],
      { windowsHide: true, maxBuffer: 16 * 1024 * 1024 },
    )
    const value = JSON.parse(stdout || "[]")
    const processes = Array.isArray(value) ? value : [value]
    for (const processInfo of processes) {
      if (!processInfo.path && processInfo.name.toLowerCase() === gameExecutableName) {
        processInfo.path = queryProcessPath(processInfo.pid)
      }
    }
    return processes
  }

  const initialProcesses = await listProcesses()
  runningProcessNames = new Set(initialProcesses.map(processInfo => processInfo.name.toLowerCase()))
  const currentSessionId = initialProcesses.find(processInfo => processInfo.pid === process.pid)?.sessionId

  function isEligibleBackground(processInfo) {
    const name = processInfo.name.toLowerCase()
    const path = normalizePath(processInfo.path)
    if (processInfo.pid <= 4 || processInfo.pid === process.pid || processInfo.sessionId !== currentSessionId) {
      return false
    }
    if (!path || path.startsWith("c:\\windows\\")) return false
    if (excludeNames.has(name) || excludePatterns.some(pattern => pattern.test(name))) return false
    return !isGame(processInfo)
  }

  async function saveState() {
    const entries = [...changed.values()].map(entry => ({
      ...entry,
      originalMask: `0x${entry.originalMask.toString(16)}`,
      appliedMask: `0x${entry.appliedMask.toString(16)}`,
    }))
    await writeFile(statePath, JSON.stringify({ entries }, null, 2), "utf8")
  }

  async function applyTo(processInfo, mask, role) {
    const key = `${processInfo.pid}:${processInfo.startTime}`
    if (handled.has(key)) return
    try {
      const originalMask = getAffinity(processInfo.pid)
      if (originalMask === mask) {
        if (!quiet) {
          console.log(`[already:${role}] ${processInfo.name} PID=${processInfo.pid}: 0x${mask.toString(16)}`)
        }
        handled.add(key)
        return
      }
      if (!quiet) {
        console.log(
          `[${role}] ${processInfo.name} PID=${processInfo.pid}: 0x${originalMask.toString(16)} -> 0x${mask.toString(16)}`,
        )
      }
      if (!applyChanges) return
      setAffinity(processInfo.pid, mask)
      changed.set(key, {
        pid: processInfo.pid,
        startTime: processInfo.startTime,
        name: processInfo.name,
        originalMask,
        appliedMask: mask,
        role,
      })
      handled.add(key)
      await saveState()
    } catch (error) {
      if (!quiet) console.warn(`[skip-permanent] ${processInfo.name} PID=${processInfo.pid}: ${error.message}`)
      handled.add(key)
    }
  }

  async function applyToGameProcesses(games) {
    if (passiveMode) return
    for (const game of games) {
      await applyTo(game, gameMask, "game")
    }
  }

  async function applyToBackgroundProcesses(processes) {
    const backgroundProcesses = processes.filter(isEligibleBackground)
    for (const backgroundProcess of backgroundProcesses) {
      await applyTo(backgroundProcess, backgroundMask, "background")
    }
  }

  function forceAffinity(processes, mask) {
    let changedCount = 0
    let alreadyCount = 0
    let skippedCount = 0

    for (const processInfo of processes) {
      try {
        const currentMask = getAffinity(processInfo.pid)
        if (currentMask === mask) {
          alreadyCount++
          continue
        }
        if (applyChanges) setAffinity(processInfo.pid, mask)
        changedCount++
      } catch (error) {
        skippedCount++
        if (!quiet) {
          console.warn(`[reorder-skip] ${processInfo.name} PID=${processInfo.pid}: ${error.message}`)
        }
      }
    }

    return { changedCount, alreadyCount, skippedCount }
  }

  async function performCpuReorder() {
    if (passiveMode || lastCoreMode) {
      throw new Error("CPU 재정렬은 기본 절반 분할 모드에서만 사용할 수 있습니다")
    }

    const temporaryProcesses = await listProcesses()
    const temporaryGames = temporaryProcesses.filter(isGame)
    if (!temporaryGames.length) throw new Error("실행 중인 마비노기를 찾을 수 없습니다")

    await applyToGameProcesses(temporaryGames)
    await applyToBackgroundProcesses(temporaryProcesses)
    const temporaryBackground = temporaryProcesses.filter(isEligibleBackground)
    const temporaryGameResult = forceAffinity(temporaryGames, lowerHalfMask)
    const temporaryBackgroundResult = forceAffinity(temporaryBackground, upperHalfMask)

    let finalGameResult = { changedCount: 0, alreadyCount: 0, skippedCount: 0 }
    let finalBackgroundResult = { changedCount: 0, alreadyCount: 0, skippedCount: 0 }
    try {
      await delay(3000)
    } finally {
      const finalProcesses = await listProcesses()
      const finalGames = finalProcesses.filter(isGame)
      await applyToGameProcesses(finalGames)
      await applyToBackgroundProcesses(finalProcesses)
      finalGameResult = forceAffinity(finalGames, upperHalfMask)
      finalBackgroundResult = forceAffinity(
        finalProcesses.filter(isEligibleBackground),
        lowerHalfMask,
      )
      gameActive = finalGames.length > 0
    }

    return {
      durationMs: 3000,
      temporary: {
        game: temporaryGameResult,
        background: temporaryBackgroundResult,
      },
      final: {
        game: finalGameResult,
        background: finalBackgroundResult,
      },
    }
  }

  async function restore(entries, processes) {
    processes ??= await listProcesses()
    const live = new Set(processes.map(processInfo => `${processInfo.pid}:${processInfo.startTime}`))
    for (const entry of entries) {
      if (!live.has(`${entry.pid}:${entry.startTime}`)) continue
      try {
        const mask = typeof entry.originalMask === "bigint" ? entry.originalMask : BigInt(entry.originalMask)
        if (!quiet) console.log(`[restore] ${entry.name} PID=${entry.pid}: 0x${mask.toString(16)}`)
        if (applyChanges) setAffinity(entry.pid, mask)
      } catch (error) {
        if (!quiet) console.warn(`[restore-skip] ${entry.name} PID=${entry.pid}: ${error.message}`)
      }
    }
  }

  async function recover() {
    try {
      const stale = JSON.parse(await readFile(statePath, "utf8"))
      if (stale.entries?.length) await restore(stale.entries)
      if (applyChanges) await unlink(statePath).catch(() => {})
    } catch (error) {
      if (error.code !== "ENOENT") throw error
    }
  }

  async function resetAllAffinities() {
    const processes = await listProcesses()
    let resetCount = 0
    let alreadyCount = 0
    let skippedCount = 0

    for (const processInfo of processes) {
      if (
        processInfo.pid <= 4
        || processInfo.pid === process.pid
        || processInfo.sessionId !== currentSessionId
      ) {
        continue
      }
      try {
        const originalMask = getAffinity(processInfo.pid)
        if (originalMask === allMask) {
          alreadyCount++
          continue
        }
        if (!quiet) {
          console.log(
            `[reset] ${processInfo.name} PID=${processInfo.pid}: 0x${originalMask.toString(16)} -> 0x${allMask.toString(16)}`,
          )
        }
        if (applyChanges) setAffinity(processInfo.pid, allMask)
        resetCount++
      } catch (error) {
        skippedCount++
        if (!quiet) console.warn(`[reset-skip] ${processInfo.name} PID=${processInfo.pid}: ${error.message}`)
      }
    }

    changed.clear()
    handled.clear()
    if (applyChanges) await unlink(statePath).catch(() => {})
    console.log(`CPU 선호도 리셋 완료: 변경 ${resetCount}, 기본값 ${alreadyCount}, 스킵 ${skippedCount}`)
  }

  async function restoreAll() {
    if (changed.size) await restore([...changed.values()])
    changed.clear()
    handled.clear()
    if (applyChanges) await unlink(statePath).catch(() => {})
  }

  async function stopWithoutRestore() {
    changed.clear()
    handled.clear()
    if (applyChanges) await unlink(statePath).catch(() => {})
  }

  async function tick() {
    const processes = await listProcesses()
    runningProcessNames = new Set(processes.map(processInfo => processInfo.name.toLowerCase()))
    const games = processes.filter(isGame)
    if (!games.length) {
      latestGameStartTime = null
      if (gameActive) {
        if (restoreOnGameExit) await restoreAll()
        else handled.clear()
        console.log(
          quiet
            ? "마비노기 프레임 부스트 Off"
            : (restoreOnGameExit
              ? "[game] Exited; restored original affinities."
              : "[game] Exited; affinity masks kept."),
        )
        gameActive = false
        return "exited"
      }
      return null
    }

    latestGameStartTime = games
      .map(game => game.startTime)
      .filter(Boolean)
      .sort()
      .at(-1) ?? null
    if (!gameActive) {
      console.log(
        quiet
          ? "마비노기 프레임 부스트 On"
          : `[game] Detected; ${allocationDescription}.`,
      )
      gameActive = true
    }
    await applyToGameProcesses(games)
    await applyToBackgroundProcesses(processes)
    return null
  }

  function printStartupSummary() {
    console.log(`nogirem ${applyChanges ? "APPLY" : "DRY-RUN"}`)
    console.log(
      `mode=${modeName}, CPUs=${logicalCpuCount}, background=0x${backgroundMask.toString(16)}, game=${gameMask === null ? "unchanged" : `0x${gameMask.toString(16)}`}`,
    )
    console.log(
      `game=auto ${gameExecutableName} under \\${config.gameDirectoryName ?? "Mabinogi"}\\ (fallback=${config.gameExecutable})`,
    )
  }

  function selfTest() {
    const ownMask = getAffinity(process.pid)
    setAffinity(process.pid, ownMask)
    console.log(`[self-test] Win32 affinity read/write passed: 0x${ownMask.toString(16)}`)
    const ownPath = queryProcessPath(process.pid)
    if (!ownPath) throw new Error("QueryFullProcessImageNameW returned no path")
    console.log(`[self-test] Native process path query passed: ${ownPath}`)
  }

  return {
    getRunningProcessNames: () => [...runningProcessNames],
    getAppliedChanges: () => [...changed.values()].map(entry => ({
      ...entry,
      originalMask: `0x${entry.originalMask.toString(16)}`,
      appliedMask: `0x${entry.appliedMask.toString(16)}`,
    })),
    hasAppliedChanges: () => changed.size > 0,
    isGameActive: () => gameActive,
    getLatestGameStartTime: () => latestGameStartTime,
    performCpuReorder,
    printStartupSummary,
    recover,
    resetAllAffinities,
    restoreAll,
    selfTest,
    stopWithoutRestore,
    tick,
  }
}

export async function hasLiveAppliedAffinityEntries(entries = []) {
  const processes = await listProcesses()
  const live = new Set(processes.map(processInfo => `${processInfo.pid}:${processInfo.startTime}`))
  for (const entry of entries) {
    if (!entry?.appliedMask || !live.has(`${entry.pid}:${entry.startTime}`)) continue
    try {
      if (getAffinity(entry.pid) === BigInt(entry.appliedMask)) return true
    } catch {
    }
  }
  return false
}
