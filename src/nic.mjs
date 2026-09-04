import { execFile } from "node:child_process"
import { mkdir, readFile, rename, unlink, writeFile } from "node:fs/promises"
import { cpus, homedir } from "node:os"
import { dirname, join } from "node:path"
import { setTimeout as delay } from "node:timers/promises"
import { promisify } from "node:util"

const execFileAsync = promisify(execFile)
const defaultStatePath = join(
  process.env.LOCALAPPDATA ?? homedir(),
  "nogirem",
  "nic-rss-state.json",
)

function encodePayload(payload) {
  return Buffer.from(JSON.stringify(payload), "utf8").toString("base64")
}

async function runNicPowerShell(action, payload = {}) {
  const encodedPayload = encodePayload(payload)
  const script = String.raw`
$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$action = "${action}"
$payloadJson = [System.Text.Encoding]::UTF8.GetString(
  [System.Convert]::FromBase64String("${encodedPayload}")
)
$payload = $payloadJson | ConvertFrom-Json

$interfaces = @{}
Get-NetIPInterface -AddressFamily IPv4 |
  Where-Object ConnectionState -eq "Connected" |
  ForEach-Object { $interfaces[$_.InterfaceIndex] = $_ }

$route = Get-NetRoute -AddressFamily IPv4 -DestinationPrefix "0.0.0.0/0" |
  Where-Object { $interfaces.ContainsKey($_.InterfaceIndex) } |
  Sort-Object @{ Expression = {
    $_.RouteMetric + $interfaces[$_.InterfaceIndex].InterfaceMetric
  } }, InterfaceIndex |
  Select-Object -First 1

if ($null -eq $route) {
  throw "연결된 IPv4 기본 경로를 찾지 못했습니다"
}

$adapter = Get-NetAdapter -InterfaceIndex $route.InterfaceIndex -ErrorAction Stop

if ($action -ne "status") {
  $principal = [Security.Principal.WindowsPrincipal]::new(
    [Security.Principal.WindowsIdentity]::GetCurrent()
  )
  if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw "NIC RSS affinity 변경에는 관리자 권한이 필요합니다"
  }

  if ($action -eq "apply") {
    $parameters = @{
      Name = $adapter.Name
      Enabled = $true
      Profile = "ClosestStatic"
      BaseProcessorGroup = [uint16]$payload.processorGroup
      BaseProcessorNumber = [byte]$payload.baseProcessorNumber
      MaxProcessorGroup = [uint16]$payload.processorGroup
      MaxProcessorNumber = [byte]$payload.maxProcessorNumber
      MaxProcessors = [uint32]$payload.maxProcessors
      NoRestart = $true
      Confirm = $false
    }
    Set-NetAdapterRss @parameters
  } elseif ($action -eq "restore") {
    if ([bool]$payload.enabled) {
      $parameters = @{
        Name = $adapter.Name
        Enabled = $true
        NoRestart = $true
        Confirm = $false
      }
      if ($null -ne $payload.profile -and [string]$payload.profile -ne "") {
        $parameters.Profile = [string]$payload.profile
      }
      if ($null -ne $payload.baseProcessorGroup) {
        $parameters.BaseProcessorGroup = [uint16]$payload.baseProcessorGroup
      }
      if ($null -ne $payload.baseProcessorNumber) {
        $parameters.BaseProcessorNumber = [byte]$payload.baseProcessorNumber
      }
      if ($null -ne $payload.maxProcessorGroup) {
        $parameters.MaxProcessorGroup = [uint16]$payload.maxProcessorGroup
      }
      if ($null -ne $payload.maxProcessorNumber) {
        $parameters.MaxProcessorNumber = [byte]$payload.maxProcessorNumber
      }
      if ($null -ne $payload.maxProcessors) {
        $parameters.MaxProcessors = [uint32]$payload.maxProcessors
      }
      Set-NetAdapterRss @parameters
    } else {
      $parameters = @{
        Name = $adapter.Name
        Enabled = $false
        NoRestart = $true
        Confirm = $false
      }
      Set-NetAdapterRss @parameters
    }
  } else {
    throw "지원하지 않는 NIC RSS 작업입니다: $action"
  }

  $adapter | Restart-NetAdapter -Confirm:$false
  $deadline = [DateTime]::UtcNow.AddSeconds(15)
  do {
    Start-Sleep -Milliseconds 500
    $adapter = Get-NetAdapter -InterfaceIndex $route.InterfaceIndex -ErrorAction Stop
  } while ($adapter.Status -ne "Up" -and [DateTime]::UtcNow -lt $deadline)

  if ($adapter.Status -ne "Up") {
    throw "15초 안에 네트워크 인터페이스가 다시 활성화되지 않았습니다"
  }
}

$rss = Get-NetAdapterRss -Name $adapter.Name -ErrorAction Stop

[pscustomobject]@{
  interfaceAlias = $adapter.Name
  interfaceIndex = [int]$adapter.InterfaceIndex
  interfaceDescription = $adapter.InterfaceDescription
  adapterStatus = [string]$adapter.Status
  enabled = [bool]$rss.Enabled
  profile = if ($null -eq $rss.Profile) { $null } else { [string]$rss.Profile }
  baseProcessorGroup = if ($null -eq $rss.BaseProcessorGroup) { $null } else { [int]$rss.BaseProcessorGroup }
  baseProcessorNumber = if ($null -eq $rss.BaseProcessorNumber) { $null } else { [int]$rss.BaseProcessorNumber }
  maxProcessorGroup = if ($null -eq $rss.MaxProcessorGroup) { $null } else { [int]$rss.MaxProcessorGroup }
  maxProcessorNumber = if ($null -eq $rss.MaxProcessorNumber) { $null } else { [int]$rss.MaxProcessorNumber }
  maxProcessors = if ($null -eq $rss.MaxProcessors) { $null } else { [int]$rss.MaxProcessors }
  numberOfReceiveQueues = if ($null -eq $rss.NumberOfReceiveQueues) { $null } else { [int]$rss.NumberOfReceiveQueues }
} | ConvertTo-Json -Compress
`

  try {
    const { stdout } = await execFileAsync(
      "powershell.exe",
      ["-NoProfile", "-NonInteractive", "-Command", script],
      { windowsHide: true, maxBuffer: 1024 * 1024, timeout: 30000 },
    )
    return JSON.parse(stdout.trim())
  } catch (error) {
    const detail = error.stderr?.trim() || error.message
    throw new Error(`NIC RSS affinity 처리 실패: ${detail}`)
  }
}

function createStateStore(statePath = defaultStatePath) {
  return {
    async load() {
      try {
        return JSON.parse(await readFile(statePath, "utf8"))
      } catch (error) {
        if (error.code === "ENOENT") return null
        throw error
      }
    },
    async save(status) {
      await mkdir(dirname(statePath), { recursive: true })
      const temporaryPath = `${statePath}.${process.pid}.tmp`
      await writeFile(temporaryPath, JSON.stringify(status, null, 2), "utf8")
      await rename(temporaryPath, statePath)
    },
    async remove() {
      await unlink(statePath).catch(error => {
        if (error.code !== "ENOENT") throw error
      })
    },
  }
}

export function buildNicRssAffinityPlan(
  status,
  { logicalCpuCount = cpus().length } = {},
) {
  if (!Number.isInteger(logicalCpuCount) || logicalCpuCount < 4 || logicalCpuCount % 2) {
    throw new Error(`지원하지 않는 논리 CPU 수입니다: ${logicalCpuCount}`)
  }
  const backgroundCpuCount = logicalCpuCount / 2
  return {
    interfaceAlias: status.interfaceAlias,
    interfaceIndex: status.interfaceIndex,
    processorGroup: 0,
    baseProcessorNumber: 0,
    maxProcessorNumber: backgroundCpuCount - 1,
    maxProcessors: backgroundCpuCount,
    gameProcessorStart: backgroundCpuCount,
    gameProcessorEnd: logicalCpuCount - 1,
    profile: "ClosestStatic",
  }
}

export function isNicRssAffinityOptimized(status, plan) {
  return status.enabled === true
    && status.baseProcessorGroup === plan.processorGroup
    && status.baseProcessorNumber === plan.baseProcessorNumber
    && status.maxProcessorGroup === plan.processorGroup
    && status.maxProcessorNumber === plan.maxProcessorNumber
    && status.maxProcessors === plan.maxProcessors
}

export async function getNicRssAffinityStatus({
  runner = runNicPowerShell,
  logicalCpuCount = cpus().length,
} = {}) {
  if (process.platform !== "win32") {
    return {
      supported: false,
      optimized: false,
      reason: "Windows에서만 NIC RSS affinity를 확인할 수 있습니다",
    }
  }
  const current = await runner("status")
  const target = buildNicRssAffinityPlan(current, { logicalCpuCount })
  return {
    supported: true,
    optimized: isNicRssAffinityOptimized(current, target),
    rssEnabled: current.enabled,
    current,
    target,
    gameCpuOverlap: current.enabled !== true
      || current.maxProcessorNumber === null
      || current.maxProcessorNumber >= target.gameProcessorStart,
  }
}

export async function applyNicRssAffinity({
  applyChanges = false,
  runner = runNicPowerShell,
  stateStore = createStateStore(),
  logicalCpuCount = cpus().length,
  wait = delay,
  verificationAttempts = 10,
  verificationDelayMs = 500,
} = {}) {
  const before = await getNicRssAffinityStatus({ runner, logicalCpuCount })
  if (!before.supported || before.optimized || !applyChanges) {
    return {
      ...before,
      applied: false,
      restarted: false,
    }
  }

  const saved = await stateStore.load()
  if (!saved) await stateStore.save(before.current)
  let current = await runner("apply", before.target)
  for (
    let attempt = 0;
    attempt < verificationAttempts && !isNicRssAffinityOptimized(current, before.target);
    attempt++
  ) {
    await wait(verificationDelayMs)
    current = await runner("status")
  }
  if (!isNicRssAffinityOptimized(current, before.target)) {
    throw new Error("NIC RSS affinity 적용 후 목표 CPU 범위를 확인하지 못했습니다")
  }
  return {
    supported: true,
    optimized: true,
    applied: true,
    restarted: true,
    before: before.current,
    current,
    target: before.target,
    gameCpuOverlap: false,
  }
}

export async function restoreNicRssAffinity({
  applyChanges = false,
  runner = runNicPowerShell,
  stateStore = createStateStore(),
} = {}) {
  if (process.platform !== "win32") {
    return {
      supported: false,
      restored: false,
      reason: "Windows에서만 NIC RSS affinity를 복원할 수 있습니다",
    }
  }

  const original = await stateStore.load()
  if (!original) {
    return {
      supported: true,
      restored: false,
      reason: "복원할 NIC RSS 원본 설정이 없습니다",
    }
  }
  if (!applyChanges) {
    return {
      supported: true,
      restored: false,
      original,
    }
  }

  const before = await runner("status")
  const alreadyRestored = original.enabled
    ? before.enabled === true
      && before.baseProcessorNumber === original.baseProcessorNumber
      && before.maxProcessorNumber === original.maxProcessorNumber
    : before.enabled === false
  if (alreadyRestored) {
    await stateStore.remove()
    return {
      supported: true,
      restored: true,
      restarted: false,
      original,
      current: before,
    }
  }

  const current = await runner("restore", original)
  const restored = original.enabled
    ? current.enabled === true
      && current.baseProcessorNumber === original.baseProcessorNumber
      && current.maxProcessorNumber === original.maxProcessorNumber
    : current.enabled === false
  if (!restored) throw new Error("NIC RSS affinity 원본 설정 복원에 실패했습니다")
  await stateStore.remove()
  return {
    supported: true,
    restored: true,
    restarted: true,
    original,
    current,
  }
}

export function printNicRssAffinityStatus(result) {
  if (!result.supported) {
    console.log(`NIC RSS affinity: 지원하지 않음 (${result.reason})`)
    return
  }
  if (result.reason) {
    console.log(`NIC RSS affinity: ${result.reason}`)
    return
  }

  const current = result.current
  const currentRange = current.enabled
    ? `${current.baseProcessorNumber ?? "자동"}-${current.maxProcessorNumber ?? "자동"}`
    : "RSS 꺼짐"
  const targetRange = result.target
    ? `${result.target.baseProcessorNumber}-${result.target.maxProcessorNumber}`
    : "없음"
  const state = result.optimized ? "최적화됨" : "조정 필요"
  console.log(
    `NIC RSS affinity: ${state} | ${current.interfaceAlias} #${current.interfaceIndex}`
      + ` | 현재=${currentRange}, 목표=${targetRange}`,
  )
}
