import { freemem, totalmem } from "node:os"
import { DataType, PointerType, createPointer, freePointer, load, open } from "ffi-rs"

open({ library: "ntdll", path: "ntdll.dll" })

const MiB = 1024 * 1024
const clamp = (value, minimum, maximum) => Math.min(maximum, Math.max(minimum, value))

function ntCall(funcName, retType, paramsType, paramsValue) {
  return load({ library: "ntdll", funcName, retType, paramsType, paramsValue })
}

function formatMiB(bytes) {
  return `${(bytes / MiB).toFixed(0)} MiB`
}

function queryStandbyBytes() {
  const buffer = Buffer.alloc(176)
  const returnLength = createPointer({ paramsType: [DataType.U32], paramsValue: [0] })
  try {
    const status = ntCall(
      "NtQuerySystemInformation",
      DataType.I32,
      [DataType.U32, DataType.U8Array, DataType.U32, DataType.External],
      [80, buffer, buffer.length, returnLength[0]],
    )
    if (status !== 0) {
      throw new Error(`NtQuerySystemInformation failed (NTSTATUS 0x${(status >>> 0).toString(16)})`)
    }
    let pages = 0n
    for (let priority = 0; priority < 8; priority++) {
      pages += buffer.readBigUInt64LE(40 + priority * 8)
    }
    return Number(pages * 4096n)
  } finally {
    freePointer({
      paramsType: [DataType.U32],
      paramsValue: returnLength,
      pointerType: PointerType.RsPointer,
    })
  }
}

function enableMemoryPurgePrivilege() {
  const previous = createPointer({ paramsType: [DataType.Boolean], paramsValue: [false] })
  try {
    const status = ntCall(
      "RtlAdjustPrivilege",
      DataType.I32,
      [DataType.U32, DataType.Boolean, DataType.Boolean, DataType.External],
      [13, true, false, previous[0]],
    )
    if (status !== 0) {
      throw new Error(`RtlAdjustPrivilege failed (NTSTATUS 0x${(status >>> 0).toString(16)})`)
    }
  } finally {
    freePointer({
      paramsType: [DataType.Boolean],
      paramsValue: previous,
      pointerType: PointerType.RsPointer,
    })
  }
}

function purgeStandbyList() {
  enableMemoryPurgePrivilege()
  const command = createPointer({ paramsType: [DataType.U32], paramsValue: [4] })
  try {
    const status = ntCall(
      "NtSetSystemInformation",
      DataType.I32,
      [DataType.U32, DataType.External, DataType.U32],
      [80, command[0], 4],
    )
    if (status !== 0) {
      throw new Error(`NtSetSystemInformation failed (NTSTATUS 0x${(status >>> 0).toString(16)})`)
    }
  } finally {
    freePointer({
      paramsType: [DataType.U32],
      paramsValue: command,
      pointerType: PointerType.RsPointer,
    })
  }
}

export function createMemoryManager({ config, applyChanges, isGameActive, isStopping }) {
  const settings = {
    pollIntervalMs: config.memoryCleaner?.pollIntervalMs ?? 1000,
    availablePercent: config.memoryCleaner?.availablePercent ?? 10,
    availableMinMiB: config.memoryCleaner?.availableMinMiB ?? 768,
    availableMaxMiB: config.memoryCleaner?.availableMaxMiB ?? 2048,
    standbyPercent: config.memoryCleaner?.standbyPercent ?? 5,
    standbyMinMiB: config.memoryCleaner?.standbyMinMiB ?? 256,
    standbyMaxMiB: config.memoryCleaner?.standbyMaxMiB ?? 1024,
    rearmPercent: config.memoryCleaner?.rearmPercent ?? 15,
    rearmMinMiB: config.memoryCleaner?.rearmMinMiB ?? 1536,
    rearmMaxMiB: config.memoryCleaner?.rearmMaxMiB ?? 4096,
    cooldownMs: config.memoryCleaner?.cooldownMs ?? 600000,
  }
  const physicalMemoryBytes = totalmem()
  const availableTriggerBytes = clamp(
    physicalMemoryBytes * settings.availablePercent / 100,
    settings.availableMinMiB * MiB,
    settings.availableMaxMiB * MiB,
  )
  const standbyTriggerBytes = clamp(
    physicalMemoryBytes * settings.standbyPercent / 100,
    settings.standbyMinMiB * MiB,
    settings.standbyMaxMiB * MiB,
  )
  const rearmAvailableBytes = clamp(
    physicalMemoryBytes * settings.rearmPercent / 100,
    settings.rearmMinMiB * MiB,
    settings.rearmMaxMiB * MiB,
  )
  let checkRunning = false
  let armed = true
  let lastCleanup = 0
  let timer = null
  let lastSnapshot = null
  let lastError = null

  function snapshot() {
    const total = totalmem()
    const available = freemem()
    const standby = queryStandbyBytes()
    lastSnapshot = { total, available, standby, availablePercent: available / total * 100 }
    return lastSnapshot
  }

  function checkMemoryPressure() {
    if (checkRunning || isStopping() || !isGameActive()) return
    checkRunning = true
    try {
      const before = snapshot()
      const recovered = before.available >= rearmAvailableBytes
      if (!armed) {
        if (recovered && Date.now() - lastCleanup >= settings.cooldownMs) armed = true
        return
      }
      const pressured = before.available < availableTriggerBytes
        && before.standby > standbyTriggerBytes
      if (!pressured || Date.now() - lastCleanup < settings.cooldownMs) return

      if (applyChanges) purgeStandbyList()
      lastCleanup = Date.now()
      armed = false
      const after = applyChanges ? snapshot() : before
      lastError = null
      console.log(
        `[memory-cleaner] ${applyChanges ? "purged" : "would purge"}: available ${formatMiB(before.available)} -> ${formatMiB(after.available)}, standby ${formatMiB(before.standby)} -> ${formatMiB(after.standby)}`,
      )
    } catch (error) {
      armed = false
      lastError = error.message
      console.error(`[memory-cleaner-error] ${error.message}`)
    } finally {
      checkRunning = false
    }
  }

  function purgeNow() {
    if (checkRunning || isStopping()) return false
    checkRunning = true
    try {
      const before = snapshot()
      if (applyChanges) purgeStandbyList()
      lastCleanup = Date.now()
      armed = false
      const after = applyChanges ? snapshot() : before
      lastError = null
      console.log(
        `[memory-cleaner] startup ${applyChanges ? "purged" : "would purge"}: available ${formatMiB(before.available)} -> ${formatMiB(after.available)}, standby ${formatMiB(before.standby)} -> ${formatMiB(after.standby)}`,
      )
      return true
    } catch (error) {
      armed = false
      lastError = error.message
      console.error(`[memory-cleaner-error] ${error.message}`)
      return false
    } finally {
      checkRunning = false
    }
  }

  function start() {
    if (timer) return
    timer = setInterval(checkMemoryPressure, settings.pollIntervalMs)
  }

  function stop() {
    if (timer) clearInterval(timer)
    timer = null
  }

  function resetForGameExit() {
    armed = true
  }

  function printStatus() {
    const current = snapshot()
    console.log(
      `memory: available=${formatMiB(current.available)} (${current.availablePercent.toFixed(1)}%), standby=${formatMiB(current.standby)}, total=${formatMiB(current.total)}`,
    )
    console.log(
      `thresholds: available<${formatMiB(availableTriggerBytes)}, standby>${formatMiB(standbyTriggerBytes)}, rearm>=${formatMiB(rearmAvailableBytes)}`,
    )
  }

  function getStartupDescription() {
    return `poll=${settings.pollIntervalMs}ms, available<${formatMiB(availableTriggerBytes)}, standby>${formatMiB(standbyTriggerBytes)}, rearm>=${formatMiB(rearmAvailableBytes)}`
  }

  function getStatus({ refresh = true } = {}) {
    const current = refresh || !lastSnapshot ? snapshot() : lastSnapshot
    return {
      running: timer !== null,
      armed,
      lastCleanup: lastCleanup || null,
      error: lastError,
      current,
      thresholds: {
        availableTrigger: availableTriggerBytes,
        standbyTrigger: standbyTriggerBytes,
        rearmAvailable: rearmAvailableBytes,
        cooldownMs: settings.cooldownMs,
      },
    }
  }

  return {
    checkNow: checkMemoryPressure,
    getStatus,
    getStartupDescription,
    purgeNow,
    printStatus,
    resetForGameExit,
    start,
    stop,
  }
}
