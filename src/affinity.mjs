import { execFile } from "node:child_process"
import { existsSync } from "node:fs"
import { promisify } from "node:util"
import { cpus } from "node:os"
import { readFile, writeFile, unlink } from "node:fs/promises"
import { basename, dirname, join } from "node:path"
import { setTimeout as delay } from "node:timers/promises"
import { DataType, PointerType, createPointer, freePointer, load, open, restorePointer } from "ffi-rs"

const execFileAsync = promisify(execFile)

open({ library: "kernel32", path: "kernel32.dll" })

const PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
const PROCESS_SET_INFORMATION = 0x0200
const ERROR_INSUFFICIENT_BUFFER = 122

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

function getProcessStartTime(pid) {
  const handle = openProcess(pid)
  if (!handle) throw new Error("OpenProcess failed")
  const pointers = Array.from({ length: 4 }, () => (
    createPointer({ paramsType: [DataType.U64], paramsValue: [0] })
  ))
  try {
    const ok = nativeCall(
      "GetProcessTimes",
      DataType.Boolean,
      [
        DataType.External,
        DataType.External,
        DataType.External,
        DataType.External,
        DataType.External,
      ],
      [handle, ...pointers.map(pointer => pointer[0])],
    )
    if (!ok) throw new Error("GetProcessTimes failed")
    const windowsEpochOffset = 116444736000000000n
    const creationTime = BigInt(readPointer(pointers[0], DataType.U64))
    return Number((creationTime - windowsEpochOffset) / 10000n)
  } finally {
    for (const pointer of pointers) {
      freePointer({ paramsType: [DataType.U64], paramsValue: pointer, pointerType: PointerType.RsPointer })
    }
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

export function getGameDirectoryNames(config) {
  const configuredNames = Array.isArray(config.gameDirectoryNames)
    ? config.gameDirectoryNames
    : [config.gameDirectoryName ?? "Mabinogi"]
  return configuredNames
    .map(value => String(value).trim().toLowerCase())
    .filter(Boolean)
}

export function matchesGameProcess(processInfo, config, fileExists = existsSync) {
  const path = normalizePath(processInfo?.path)
  const executableName = String(
    config.gameExecutableName ?? basename(config.gameExecutable ?? "Client.exe"),
  ).toLowerCase()
  if (!path || String(processInfo?.name).toLowerCase() !== executableName) return false
  if (normalizePath(config.gameExecutable) === path) return true
  const parentDirectoryName = path.split("\\").filter(Boolean).at(-2)
  if (getGameDirectoryNames(config).includes(parentDirectoryName)) return true
  return fileExists(join(dirname(String(processInfo.path)), "Mabinogi.exe"))
}

function maskFromCpuIndexes(cpuIndexes) {
  return cpuIndexes.reduce((mask, cpuIndex) => mask | (1n << BigInt(cpuIndex)), 0n)
}

function formatCpuIndexes(cpuIndexes) {
  if (!cpuIndexes.length) return "없음"
  const ranges = []
  let start = cpuIndexes[0]
  let end = start
  for (const cpuIndex of cpuIndexes.slice(1)) {
    if (cpuIndex === end + 1) {
      end = cpuIndex
      continue
    }
    ranges.push(start === end ? `${start}` : `${start}-${end}`)
    start = cpuIndex
    end = cpuIndex
  }
  ranges.push(start === end ? `${start}` : `${start}-${end}`)
  return ranges.join(",")
}

export function defaultGamePhysicalCoreCount(performanceCoreCount) {
  if (!Number.isInteger(performanceCoreCount) || performanceCoreCount < 1) {
    throw new Error(`Invalid performance core count: ${performanceCoreCount}`)
  }
  if (performanceCoreCount === 1) return 1
  const half = Math.ceil(performanceCoreCount / 2)
  return Math.min(performanceCoreCount - 1, Math.max(4, half))
}

export function normalizeGamePhysicalCoreCount(value, performanceCoreCount) {
  const defaultCount = defaultGamePhysicalCoreCount(performanceCoreCount)
  if (value === null || value === undefined || value === "") return defaultCount
  const parsed = Number(value)
  if (!Number.isInteger(parsed)) return defaultCount
  return Math.min(Math.max(parsed, 1), Math.max(performanceCoreCount - 1, 1))
}

export function buildCpuTopologyMasks(cpuSets, logicalCpuCount, requestedGameCoreCount = null) {
  if (logicalCpuCount < 4 || logicalCpuCount > 52 || logicalCpuCount % 2) {
    throw new Error(`Unsupported logical CPU count: ${logicalCpuCount}; expected an even count from 4 to 52.`)
  }
  const processors = cpuSets
    .filter(cpuSet => cpuSet.group === 0)
    .sort((left, right) => left.logicalProcessorIndex - right.logicalProcessorIndex)
  const indexes = processors.map(cpuSet => cpuSet.logicalProcessorIndex)
  if (
    processors.length !== logicalCpuCount
    || new Set(indexes).size !== logicalCpuCount
    || indexes.some((cpuIndex, index) => cpuIndex !== index)
  ) {
    throw new Error("CPU Set topology does not match the process affinity group.")
  }

  const physicalCores = new Map()
  for (const processor of processors) {
    const key = `${processor.group}:${processor.coreIndex}`
    const core = physicalCores.get(key) ?? {
      efficiencyClass: processor.efficiencyClass,
      cpuIndexes: [],
    }
    if (core.efficiencyClass !== processor.efficiencyClass) {
      throw new Error(`Inconsistent efficiency class for physical core ${key}.`)
    }
    core.cpuIndexes.push(processor.logicalProcessorIndex)
    physicalCores.set(key, core)
  }

  const cores = [...physicalCores.values()]
    .map(core => ({ ...core, cpuIndexes: core.cpuIndexes.sort((left, right) => left - right) }))
    .sort((left, right) => left.cpuIndexes[0] - right.cpuIndexes[0])
  const performanceClass = Math.max(...cores.map(core => core.efficiencyClass))
  const performanceCores = cores.filter(core => core.efficiencyClass === performanceClass)
  const efficiencyCores = cores.filter(core => core.efficiencyClass < performanceClass)
  const hybrid = efficiencyCores.length > 0
  const defaultGameCoreCount = defaultGamePhysicalCoreCount(performanceCores.length)
  const gameCoreCount = normalizeGamePhysicalCoreCount(
    requestedGameCoreCount,
    performanceCores.length,
  )
  const gameCores = performanceCores.slice(-gameCoreCount)
  const backgroundPerformanceCores = performanceCores.slice(0, -gameCoreCount)
  const alternateGameCores = backgroundPerformanceCores
  const alternateBackgroundCores = [...gameCores, ...efficiencyCores]
  const gameCpuIndexes = gameCores.flatMap(core => core.cpuIndexes).sort((left, right) => left - right)
  const backgroundCpuIndexes = [...backgroundPerformanceCores, ...efficiencyCores]
    .flatMap(core => core.cpuIndexes)
    .sort((left, right) => left - right)
  const alternateGameCpuIndexes = alternateGameCores
    .flatMap(core => core.cpuIndexes)
    .sort((left, right) => left - right)
  const alternateBackgroundCpuIndexes = alternateBackgroundCores
    .flatMap(core => core.cpuIndexes)
    .sort((left, right) => left - right)

  return {
    source: "windows-cpu-sets",
    allMask: maskFromCpuIndexes(indexes),
    gameMask: maskFromCpuIndexes(gameCpuIndexes),
    backgroundMask: maskFromCpuIndexes(backgroundCpuIndexes),
    alternateGameMask: maskFromCpuIndexes(alternateGameCpuIndexes),
    alternateBackgroundMask: maskFromCpuIndexes(alternateBackgroundCpuIndexes),
    lastPerformanceCoreMask: maskFromCpuIndexes(performanceCores.at(-1).cpuIndexes),
    gameCpuIndexes,
    backgroundCpuIndexes,
    physicalCoreCount: cores.length,
    performanceCoreCount: performanceCores.length,
    efficiencyCoreCount: efficiencyCores.length,
    gameCoreCount,
    defaultGameCoreCount,
    maxGameCoreCount: Math.max(performanceCores.length - 1, 1),
    hybrid,
  }
}

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

function queryWindowsCpuSets() {
  const returnedLength = createPointer({ paramsType: [DataType.U32], paramsValue: [0] })
  const currentProcess = nativeCall("GetCurrentProcess", DataType.External, [], [])
  try {
    const initialBuffer = Buffer.alloc(1)
    const initialResult = nativeCall(
      "GetSystemCpuSetInformation",
      DataType.Boolean,
      [DataType.U8Array, DataType.U32, DataType.External, DataType.External, DataType.U32],
      [initialBuffer, 0, returnedLength[0], currentProcess, 0],
    )
    const requiredLength = Number(readPointer(returnedLength, DataType.U32))
    if (!initialResult && lastError() !== ERROR_INSUFFICIENT_BUFFER) {
      throw new Error(`GetSystemCpuSetInformation size query failed (Win32 ${lastError()})`)
    }
    if (!requiredLength) throw new Error("GetSystemCpuSetInformation returned no CPU Sets.")

    const buffer = Buffer.alloc(requiredLength)
    const ok = nativeCall(
      "GetSystemCpuSetInformation",
      DataType.Boolean,
      [DataType.U8Array, DataType.U32, DataType.External, DataType.External, DataType.U32],
      [buffer, buffer.length, returnedLength[0], currentProcess, 0],
    )
    if (!ok) throw new Error(`GetSystemCpuSetInformation failed (Win32 ${lastError()})`)

    const cpuSets = []
    const validLength = Number(readPointer(returnedLength, DataType.U32))
    for (let offset = 0; offset + 8 <= validLength;) {
      const size = buffer.readUInt32LE(offset)
      const type = buffer.readUInt32LE(offset + 4)
      if (size < 8 || offset + size > validLength) {
        throw new Error("GetSystemCpuSetInformation returned malformed data.")
      }
      if (type === 0 && size >= 32) {
        cpuSets.push({
          id: buffer.readUInt32LE(offset + 8),
          group: buffer.readUInt16LE(offset + 12),
          logicalProcessorIndex: buffer.readUInt8(offset + 14),
          coreIndex: buffer.readUInt8(offset + 15),
          efficiencyClass: buffer.readUInt8(offset + 18),
        })
      }
      offset += size
    }
    return cpuSets
  } finally {
    freePointer({
      paramsType: [DataType.U32],
      paramsValue: returnedLength,
      pointerType: PointerType.RsPointer,
    })
  }
}

export function resolveCpuAllocation(
  logicalCpuCount = cpus().length,
  { gameCoreCount = null } = {},
) {
  if (process.platform !== "win32" || process.arch !== "x64") {
    throw new Error("Windows x64 only.")
  }
  try {
    return buildCpuTopologyMasks(queryWindowsCpuSets(), logicalCpuCount, gameCoreCount)
  } catch (error) {
    const { allMask, lowerHalfMask, upperHalfMask } = buildCpuHalfMasks(logicalCpuCount)
    const half = logicalCpuCount / 2
    return {
      source: "logical-half-fallback",
      topologyError: {
        message: error?.message ?? String(error),
        logicalCpuCount,
      },
      allMask,
      gameMask: upperHalfMask,
      backgroundMask: lowerHalfMask,
      alternateGameMask: lowerHalfMask,
      alternateBackgroundMask: upperHalfMask,
      lastPerformanceCoreMask: 3n << BigInt(logicalCpuCount - 2),
      gameCpuIndexes: Array.from({ length: half }, (_, index) => index + half),
      backgroundCpuIndexes: Array.from({ length: half }, (_, index) => index),
      physicalCoreCount: null,
      performanceCoreCount: null,
      efficiencyCoreCount: null,
      gameCoreCount: null,
      defaultGameCoreCount: null,
      maxGameCoreCount: null,
      hybrid: null,
    }
  }
}

export function pinCurrentProcessToBackgroundCpus({ gameCoreCount = null } = {}) {
  const logicalCpuCount = cpus().length
  const allocation = resolveCpuAllocation(logicalCpuCount, { gameCoreCount })
  const { backgroundMask } = allocation
  setAffinity(process.pid, backgroundMask)
  return {
    mask: `0x${backgroundMask.toString(16)}`,
    cpuRange: formatCpuIndexes(allocation.backgroundCpuIndexes),
    source: allocation.source,
    hybrid: allocation.hybrid,
    physicalCoreCount: allocation.physicalCoreCount,
    performanceCoreCount: allocation.performanceCoreCount,
    efficiencyCoreCount: allocation.efficiencyCoreCount,
    gameCoreCount: allocation.gameCoreCount,
    defaultGameCoreCount: allocation.defaultGameCoreCount,
    maxGameCoreCount: allocation.maxGameCoreCount,
    backgroundCpuRange: formatCpuIndexes(allocation.backgroundCpuIndexes),
    gameCpuRange: formatCpuIndexes(allocation.gameCpuIndexes),
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
  gameCoreCount = null,
}) {
  const logicalCpuCount = cpus().length

  if (process.platform !== "win32" || process.arch !== "x64") throw new Error("Windows x64 only.")
  const allocation = resolveCpuAllocation(logicalCpuCount, { gameCoreCount })
  const { allMask } = allocation
  const gameMask = passiveMode
    ? null
    : (lastCoreMode ? allocation.lastPerformanceCoreMask : allocation.gameMask)
  const backgroundMask = passiveMode ? allocation.gameMask : allMask ^ gameMask
  const latencyMask = allocation.alternateGameMask || backgroundMask
  const modeName = passiveMode ? "passive" : (lastCoreMode ? "last-core" : "half")
  const allocationDescription = passiveMode
    ? `game affinity unchanged, background CPUs ${formatCpuIndexes(allocation.gameCpuIndexes)}`
    : (lastCoreMode
      ? `background CPUs 0-${logicalCpuCount - 3}, game CPUs ${logicalCpuCount - 2}-${logicalCpuCount - 1}`
      : `background CPUs ${formatCpuIndexes(allocation.backgroundCpuIndexes)}, game P-core CPUs ${formatCpuIndexes(allocation.gameCpuIndexes)}`)
  const gameExecutableName = String(
    config.gameExecutableName ?? basename(config.gameExecutable ?? "Client.exe"),
  ).toLowerCase()
  const excludeNames = new Set(config.excludeNames.map(value => value.toLowerCase()))
  const excludePatterns = config.excludeNamePatterns.map(value => new RegExp(value, "i"))
  const latencyNames = new Set(
    (config.latencySensitiveNames ?? []).map(value => value.toLowerCase()),
  )
  const latencyPatterns = (config.latencySensitiveNamePatterns ?? [])
    .map(value => new RegExp(value, "i"))
  const changed = new Map()
  const handled = new Set()
  let gameActive = false
  let latestGameStartTime = null
  let latestGameExecutablePath = null
  let runningProcessNames = new Set()

  const isGame = processInfo => matchesGameProcess(processInfo, config)

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

  function isEligibleManagedProcess(processInfo) {
    const name = processInfo.name.toLowerCase()
    const path = normalizePath(processInfo.path)
    if (processInfo.pid <= 4 || processInfo.pid === process.pid || processInfo.sessionId !== currentSessionId) {
      return false
    }
    if (!path || path.startsWith("c:\\windows\\")) return false
    if (excludeNames.has(name) || excludePatterns.some(pattern => pattern.test(name))) return false
    return !isGame(processInfo)
  }

  function isLatencySensitive(processInfo) {
    if (!isEligibleManagedProcess(processInfo)) return false
    const name = processInfo.name.toLowerCase()
    return latencyNames.has(name) || latencyPatterns.some(pattern => pattern.test(name))
  }

  function isEligibleBackground(processInfo) {
    return isEligibleManagedProcess(processInfo) && !isLatencySensitive(processInfo)
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

  async function applyToLatencySensitiveProcesses(processes) {
    const latencyProcesses = processes.filter(isLatencySensitive)
    for (const latencyProcess of latencyProcesses) {
      await applyTo(latencyProcess, latencyMask, "latency")
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
    await applyToLatencySensitiveProcesses(temporaryProcesses)
    await applyToBackgroundProcesses(temporaryProcesses)
    const temporaryBackground = temporaryProcesses.filter(isEligibleBackground)
    const temporaryLatency = temporaryProcesses.filter(isLatencySensitive)
    if (!allocation.alternateGameMask) {
      throw new Error("CPU 재정렬에 사용할 대체 P-core 그룹이 없습니다")
    }
    const temporaryGameResult = forceAffinity(temporaryGames, allocation.alternateGameMask)
    const temporaryBackgroundResult = forceAffinity(
      temporaryBackground,
      allocation.alternateBackgroundMask,
    )
    const temporaryLatencyResult = forceAffinity(temporaryLatency, allocation.gameMask)

    let finalGameResult = { changedCount: 0, alreadyCount: 0, skippedCount: 0 }
    let finalBackgroundResult = { changedCount: 0, alreadyCount: 0, skippedCount: 0 }
    try {
      await delay(3000)
    } finally {
      const finalProcesses = await listProcesses()
      const finalGames = finalProcesses.filter(isGame)
      await applyToGameProcesses(finalGames)
      await applyToLatencySensitiveProcesses(finalProcesses)
      await applyToBackgroundProcesses(finalProcesses)
      finalGameResult = forceAffinity(finalGames, allocation.gameMask)
      finalBackgroundResult = forceAffinity(
        finalProcesses.filter(isEligibleBackground),
        allocation.backgroundMask,
      )
      forceAffinity(finalProcesses.filter(isLatencySensitive), latencyMask)
      gameActive = finalGames.length > 0
    }

    return {
      durationMs: 3000,
      temporary: {
        game: temporaryGameResult,
        background: temporaryBackgroundResult,
        latency: temporaryLatencyResult,
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
    latestGameExecutablePath = games
      .filter(game => game.path)
      .sort((left, right) => String(left.startTime ?? "").localeCompare(String(right.startTime ?? "")))
      .at(-1)?.path ?? latestGameExecutablePath
    if (!gameActive) {
      console.log(
        quiet
          ? "마비노기 프레임 부스트 On"
          : `[game] Detected; ${allocationDescription}.`,
      )
      gameActive = true
    }
    await applyToGameProcesses(games)
    await applyToLatencySensitiveProcesses(processes)
    await applyToBackgroundProcesses(processes)
    return null
  }

  function printStartupSummary() {
    console.log(`nogirem ${applyChanges ? "APPLY" : "DRY-RUN"}`)
    console.log(
      `mode=${modeName}, CPUs=${logicalCpuCount}, background=0x${backgroundMask.toString(16)}, game=${gameMask === null ? "unchanged" : `0x${gameMask.toString(16)}`}`,
    )
    console.log(
      `game=auto ${gameExecutableName} under ${getGameDirectoryNames(config).join(", ")} (fallback=${config.gameExecutable})`,
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
    getLatestGameExecutablePath: () => latestGameExecutablePath,
    getCpuAllocation: () => ({
      source: allocation.source,
      hybrid: allocation.hybrid,
      physicalCoreCount: allocation.physicalCoreCount,
      performanceCoreCount: allocation.performanceCoreCount,
      efficiencyCoreCount: allocation.efficiencyCoreCount,
      gameCoreCount: allocation.gameCoreCount,
      defaultGameCoreCount: allocation.defaultGameCoreCount,
      maxGameCoreCount: allocation.maxGameCoreCount,
      backgroundCpuRange: formatCpuIndexes(allocation.backgroundCpuIndexes),
      gameCpuRange: formatCpuIndexes(allocation.gameCpuIndexes),
    }),
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

export async function hasLiveAppliedAffinityEntries(entries = [], {
  processStartReader = getProcessStartTime,
  affinityReader = getAffinity,
} = {}) {
  const applicableEntries = entries.filter(entry => entry?.appliedMask)
  if (!applicableEntries.length) return false
  for (const entry of applicableEntries) {
    try {
      if (processStartReader(entry.pid) !== Date.parse(entry.startTime)) continue
      if (affinityReader(entry.pid) === BigInt(entry.appliedMask)) return true
    } catch {
    }
  }
  return false
}
