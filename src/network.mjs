import { execFile } from "node:child_process"
import { setTimeout as delay } from "node:timers/promises"
import { promisify } from "node:util"

const execFileAsync = promisify(execFile)

const fastPingValues = {
  TcpAckFrequency: 1,
  TCPNoDelay: 1,
}

const queryScript = String.raw`
$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
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

$adapter = Get-NetAdapter -IncludeHidden -ErrorAction SilentlyContinue |
  Where-Object InterfaceIndex -eq $route.InterfaceIndex |
  Select-Object -First 1

if ($null -eq $adapter) {
  [pscustomobject]@{
    supported = $false
    specialNetwork = $true
    reason = "특수 네트워크 환경으로 패스트핑 적용 생략"
  } | ConvertTo-Json -Compress
  return
}

$guidValue = $adapter.InterfaceGuid.ToString().Trim("{}")
$guid = "{$guidValue}"
$registryPath = "HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces\$guid"
$thirdPartyBindings = @(
  Get-NetAdapterBinding -Name $adapter.Name -AllBindings -ErrorAction SilentlyContinue |
    Where-Object { $_.Enabled -and $_.ComponentID -notmatch "^ms_" } |
    Select-Object -ExpandProperty ComponentID
)
$captureBindings = @(
  $thirdPartyBindings |
    Where-Object { $_ -match "(?i)^(insecure_npcap|npcap(?:_wifi)?)$" }
)
$blockingThirdPartyBindings = @(
  $thirdPartyBindings |
    Where-Object { $_ -notmatch "(?i)^(insecure_npcap|npcap(?:_wifi)?)$" }
)
$isVirtual = (
  -not [bool]$adapter.HardwareInterface -or
  $adapter.InterfaceDescription -match "(?i)virtual|vpn|tap|tun|wintun|wireguard|hyper-v|vmware|virtualbox"
)
$compatible = -not $isVirtual -and $blockingThirdPartyBindings.Count -eq 0
$compatibilityReason = if ($isVirtual) {
  "VPN 또는 가상 네트워크 어댑터에는 패스트핑을 적용하지 않습니다"
} elseif ($blockingThirdPartyBindings.Count -gt 0) {
  "타사 네트워크 필터가 연결된 어댑터에는 패스트핑을 적용하지 않습니다"
} else {
  $null
}

if ($applyFastPing) {
  $principal = [Security.Principal.WindowsPrincipal]::new(
    [Security.Principal.WindowsIdentity]::GetCurrent()
  )
  if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw "패스트핑 적용에는 관리자 권한이 필요합니다"
  }
  New-Item -Path $registryPath -Force | Out-Null
  New-ItemProperty -LiteralPath $registryPath -Name "TcpAckFrequency" -PropertyType DWord -Value 1 -Force | Out-Null
  New-ItemProperty -LiteralPath $registryPath -Name "TCPNoDelay" -PropertyType DWord -Value 1 -Force | Out-Null
}

$item = Get-ItemProperty -LiteralPath $registryPath -ErrorAction SilentlyContinue
$ackFrequency = if (
  $null -ne $item -and
  $item.PSObject.Properties.Name -contains "TcpAckFrequency"
) { [uint32]$item.TcpAckFrequency } else { $null }
$noDelay = if (
  $null -ne $item -and
  $item.PSObject.Properties.Name -contains "TCPNoDelay"
) { [uint32]$item.TCPNoDelay } else { $null }

[pscustomobject]@{
  supported = $true
  interfaceAlias = $adapter.Name
  interfaceIndex = $route.InterfaceIndex
  interfaceGuid = $guid
  nextHop = $route.NextHop
  routeMetric = $route.RouteMetric
  interfaceMetric = $interfaces[$route.InterfaceIndex].InterfaceMetric
  compatible = $compatible
  compatibilityReason = $compatibilityReason
  virtualAdapter = $isVirtual
  thirdPartyBindings = $thirdPartyBindings
  captureBindings = $captureBindings
  blockingThirdPartyBindings = $blockingThirdPartyBindings
  TcpAckFrequency = $ackFrequency
  TCPNoDelay = $noDelay
} | ConvertTo-Json -Compress
`

async function runPowerShell(applyFastPing) {
  const script = `$applyFastPing = $${applyFastPing ? "true" : "false"}\n${queryScript}`
  try {
    const { stdout } = await execFileAsync(
      "powershell.exe",
      ["-NoProfile", "-NonInteractive", "-Command", script],
      { windowsHide: true, maxBuffer: 1024 * 1024 },
    )
    return JSON.parse(stdout.trim())
  } catch (error) {
    const detail = error.stderr?.trim() || error.message
    throw new Error(`주 네트워크 인터페이스의 패스트핑 처리 실패: ${detail}`)
  }
}

async function runConnectivityPowerShell() {
  const script = String.raw`
$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

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

$validIpv4 = $false
$defaultRoute = $null -ne $route
$gateway = $false
$interfaceIndex = if ($defaultRoute) { [int]$route.InterfaceIndex } else { 0 }
if ($defaultRoute) {
  $addresses = @(Get-NetIPAddress -AddressFamily IPv4 -InterfaceIndex $route.InterfaceIndex -ErrorAction SilentlyContinue)
  $validIpv4 = @(
    $addresses | Where-Object {
      $_.IPAddress -notmatch "^169\.254\." -and
      $_.IPAddress -ne "0.0.0.0"
    }
  ).Count -gt 0
  $gateway = -not [string]::IsNullOrWhiteSpace([string]$route.NextHop) -and
    [string]$route.NextHop -ne "0.0.0.0"
}

$dns = $false
foreach ($hostName in @("www.microsoft.com", "api.github.com")) {
  try {
    $dns = $null -ne (
      Resolve-DnsName $hostName -DnsOnly -QuickTimeout -ErrorAction Stop |
        Select-Object -First 1
    )
    if ($dns) { break }
  } catch {}
}

$https = $false
foreach ($endpoint in @("https://www.microsoft.com", "https://api.github.com")) {
  try {
    Invoke-WebRequest -Uri $endpoint -Method Head -UseBasicParsing -TimeoutSec 5 -ErrorAction Stop |
      Out-Null
    $https = $true
    break
  } catch {}
}

[pscustomobject]@{
  validIpv4 = $validIpv4
  defaultRoute = $defaultRoute
  gateway = $gateway
  dns = $dns
  https = $https
  interfaceIndex = $interfaceIndex
  nextHop = if ($defaultRoute) { [string]$route.NextHop } else { $null }
} | ConvertTo-Json -Compress
`
  try {
    const { stdout } = await execFileAsync(
      "powershell.exe",
      ["-NoProfile", "-NonInteractive", "-Command", script],
      { windowsHide: true, maxBuffer: 1024 * 1024, timeout: 15000 },
    )
    return JSON.parse(stdout.trim())
  } catch (error) {
    const detail = error.stderr?.trim() || error.message
    return {
      validIpv4: false,
      defaultRoute: false,
      gateway: false,
      dns: false,
      https: false,
      error: detail,
    }
  }
}

export async function checkNetworkConnectivity({
  attempts = 1,
  intervalMs = 3000,
  runner = runConnectivityPowerShell,
} = {}) {
  const maximumAttempts = Math.max(1, Math.min(20, Math.round(Number(attempts) || 1)))
  let current = null
  for (let attempt = 0; attempt < maximumAttempts; attempt += 1) {
    current = await runner()
    const healthy = Boolean(
      current?.validIpv4
      && current?.defaultRoute
      && current?.gateway
      && current?.dns
      && current?.https,
    )
    if (healthy) {
      return { healthy: true, attempts: attempt + 1, current }
    }
    if (attempt + 1 < maximumAttempts) await delay(intervalMs)
  }
  return { healthy: false, attempts: maximumAttempts, current }
}

async function runFastPingRestorePowerShell(target) {
  const interfaceGuid = String(target?.interfaceGuid ?? "")
  if (!/^\{[0-9a-f-]{36}\}$/i.test(interfaceGuid)) {
    throw new Error("복원할 네트워크 인터페이스 GUID가 올바르지 않습니다")
  }
  const interfaceIndex = Number(target?.interfaceIndex)
  if (!Number.isInteger(interfaceIndex) || interfaceIndex < 1) {
    throw new Error("복원할 네트워크 인터페이스 인덱스가 올바르지 않습니다")
  }
  const valueLiteral = value => {
    if (value === null || value === undefined) return "$null"
    const number = Number(value)
    if (!Number.isInteger(number) || number < 0 || number > 0xffffffff) {
      throw new Error("복원할 패스트핑 값이 올바르지 않습니다")
    }
    return `[uint32]${number}`
  }
  const script = String.raw`
$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$principal = [Security.Principal.WindowsPrincipal]::new(
  [Security.Principal.WindowsIdentity]::GetCurrent()
)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
  throw "패스트핑 복원에는 관리자 권한이 필요합니다"
}

$guid = "${interfaceGuid}"
$registryPath = "HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces\$guid"
New-Item -Path $registryPath -Force | Out-Null

function Restore-DwordValue($name, $value) {
  if ($null -eq $value) {
    Remove-ItemProperty -LiteralPath $registryPath -Name $name -ErrorAction SilentlyContinue
  } else {
    New-ItemProperty -LiteralPath $registryPath -Name $name -PropertyType DWord -Value $value -Force | Out-Null
  }
}

Restore-DwordValue "TcpAckFrequency" ${valueLiteral(target.TcpAckFrequency)}
Restore-DwordValue "TCPNoDelay" ${valueLiteral(target.TCPNoDelay)}

$item = Get-ItemProperty -LiteralPath $registryPath -ErrorAction SilentlyContinue
$ackFrequency = if (
  $null -ne $item -and
  $item.PSObject.Properties.Name -contains "TcpAckFrequency"
) { [uint32]$item.TcpAckFrequency } else { $null }
$noDelay = if (
  $null -ne $item -and
  $item.PSObject.Properties.Name -contains "TCPNoDelay"
) { [uint32]$item.TCPNoDelay } else { $null }
$adapter = Get-NetAdapter -IncludeHidden -ErrorAction SilentlyContinue |
  Where-Object { $_.InterfaceGuid.ToString().Trim("{}") -eq $guid.Trim("{}") } |
  Select-Object -First 1

[pscustomobject]@{
  supported = $true
  interfaceAlias = if ($null -ne $adapter) { $adapter.Name } else { "기본 네트워크" }
  interfaceIndex = if ($null -ne $adapter) { $adapter.InterfaceIndex } else { ${interfaceIndex} }
  interfaceGuid = $guid
  TcpAckFrequency = $ackFrequency
  TCPNoDelay = $noDelay
} | ConvertTo-Json -Compress
`

  try {
    const { stdout } = await execFileAsync(
      "powershell.exe",
      ["-NoProfile", "-NonInteractive", "-Command", script],
      { windowsHide: true, maxBuffer: 1024 * 1024 },
    )
    return JSON.parse(stdout.trim())
  } catch (error) {
    const detail = error.stderr?.trim() || error.message
    throw new Error(`패스트핑 설정 복원 실패: ${detail}`)
  }
}

async function runAdapterRestart(status) {
  const interfaceIndex = Number(status.interfaceIndex)
  if (!Number.isInteger(interfaceIndex) || interfaceIndex < 1) {
    throw new Error(`잘못된 네트워크 인터페이스 인덱스: ${status.interfaceIndex}`)
  }

  const script = String.raw`
$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$principal = [Security.Principal.WindowsPrincipal]::new(
  [Security.Principal.WindowsIdentity]::GetCurrent()
)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
  throw "네트워크 인터페이스 재시작에는 관리자 권한이 필요합니다"
}

$adapter = Get-NetAdapter -InterfaceIndex ${interfaceIndex} -ErrorAction Stop
$adapter | Restart-NetAdapter -Confirm:$false

$deadline = [DateTime]::UtcNow.AddSeconds(15)
do {
  Start-Sleep -Milliseconds 500
  $adapter = Get-NetAdapter -InterfaceIndex ${interfaceIndex} -ErrorAction Stop
} while ($adapter.Status -ne "Up" -and [DateTime]::UtcNow -lt $deadline)

if ($adapter.Status -ne "Up") {
  throw "15초 안에 네트워크 인터페이스가 다시 활성화되지 않았습니다"
}

[pscustomobject]@{
  interfaceAlias = $adapter.Name
  interfaceIndex = $adapter.InterfaceIndex
  status = $adapter.Status
} | ConvertTo-Json -Compress
`

  try {
    const { stdout } = await execFileAsync(
      "powershell.exe",
      ["-NoProfile", "-NonInteractive", "-Command", script],
      { windowsHide: true, maxBuffer: 1024 * 1024 },
    )
    return JSON.parse(stdout.trim())
  } catch (error) {
    const detail = error.stderr?.trim() || error.message
    throw new Error(`주 네트워크 인터페이스 재시작 실패: ${detail}`)
  }
}

async function runAutoTuningPowerShell(applyNormal) {
  const script = String.raw`
$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()

if ($applyNormal) {
  $principal = [Security.Principal.WindowsPrincipal]::new(
    [Security.Principal.WindowsIdentity]::GetCurrent()
  )
  if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw "TCP 자동 조정 수준 복구에는 관리자 권한이 필요합니다"
  }
  & netsh.exe interface tcp set global autotuninglevel=normal | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw "netsh가 종료 코드 $LASTEXITCODE을 반환했습니다"
  }
}

$setting = Get-NetTCPSetting -SettingName "Internet" -ErrorAction Stop
$local = [string]$setting.AutoTuningLevelLocal
$groupPolicy = [string]$setting.AutoTuningLevelGroupPolicy
$effective = if ($groupPolicy -eq "NotConfigured") { $local } else { $groupPolicy }

[pscustomobject]@{
  settingName = $setting.SettingName
  local = $local
  groupPolicy = $groupPolicy
  effective = $effective
} | ConvertTo-Json -Compress
`

  const command = `$applyNormal = $${applyNormal ? "true" : "false"}\n${script}`
  try {
    const { stdout } = await execFileAsync(
      "powershell.exe",
      ["-NoProfile", "-NonInteractive", "-Command", command],
      { windowsHide: true, maxBuffer: 1024 * 1024 },
    )
    return JSON.parse(stdout.trim())
  } catch (error) {
    const detail = error.stderr?.trim() || error.message
    throw new Error(`TCP 수신 창 자동 조정 처리 실패: ${detail}`)
  }
}

export function isFastPingConfigured(status) {
  return status?.supported !== false
    && Object.entries(fastPingValues).every(([name, expected]) => status[name] === expected)
}

export function isTcpAutoTuningNormal(status) {
  return String(status.effective).toLowerCase() === "normal"
}

export async function ensureTcpAutoTuningNormal({
  applyChanges = false,
  runner = runAutoTuningPowerShell,
} = {}) {
  if (process.platform !== "win32") {
    return {
      supported: false,
      optimized: false,
      applied: false,
      reason: "Windows에서만 TCP 자동 조정 수준을 설정할 수 있습니다",
    }
  }

  const before = await runner(false)
  if (isTcpAutoTuningNormal(before)) {
    return {
      supported: true,
      optimized: true,
      applied: false,
      before,
      current: before,
    }
  }

  if (!applyChanges) {
    return {
      supported: true,
      optimized: false,
      applied: false,
      before,
      current: before,
    }
  }

  const current = await runner(true)
  if (!isTcpAutoTuningNormal(current)) {
    const policy = current.groupPolicy !== "NotConfigured"
      ? `, 그룹 정책=${current.groupPolicy}`
      : ""
    throw new Error(`TCP 자동 조정 수준을 Normal로 복구하지 못했습니다${policy}`)
  }

  return {
    supported: true,
    optimized: true,
    applied: true,
    before,
    current,
  }
}

export async function restartPrimaryNetworkInterface(
  status,
  { restarter = runAdapterRestart } = {},
) {
  if (process.platform !== "win32") {
    throw new Error("Windows에서만 네트워크 인터페이스를 재시작할 수 있습니다")
  }
  return restarter(status)
}

function sameFastPingValue(current, expected) {
  return (current ?? null) === (expected ?? null)
}

export async function restoreFastPingForInterface(
  target,
  {
    restartAfterRestore = false,
    runner = runFastPingRestorePowerShell,
    restarter = runAdapterRestart,
  } = {},
) {
  if (process.platform !== "win32") {
    return {
      supported: false,
      restored: false,
      restarted: false,
      reason: "Windows에서만 패스트핑 설정을 복원할 수 있습니다",
    }
  }

  const current = await runner(target)
  if (
    !sameFastPingValue(current.TcpAckFrequency, target.TcpAckFrequency)
    || !sameFastPingValue(current.TCPNoDelay, target.TCPNoDelay)
  ) {
    throw new Error("패스트핑 레지스트리 값을 복원한 뒤 검증에 실패했습니다")
  }
  const restart = restartAfterRestore
    ? await restartPrimaryNetworkInterface(current, { restarter })
    : null
  return {
    supported: true,
    configured: isFastPingConfigured(current),
    restored: true,
    restarted: restart !== null,
    restart,
    target,
    current,
  }
}

export async function ensureFastPingForPrimaryInterface({
  applyChanges = false,
  restartAfterApply = false,
  runner = runPowerShell,
  restarter = runAdapterRestart,
} = {}) {
  if (process.platform !== "win32") {
    return {
      supported: false,
      configured: false,
      applied: false,
      reason: "Windows에서만 패스트핑을 설정할 수 있습니다",
    }
  }

  const before = await runner(false)
  if (before?.supported === false) {
    return {
      supported: false,
      configured: false,
      applied: false,
      restarted: false,
      reason: before.reason ?? "특수 네트워크 환경으로 패스트핑 적용 생략",
      before,
      current: before,
    }
  }
  if (isFastPingConfigured(before)) {
    return {
      supported: true,
      configured: true,
      applied: false,
      restarted: false,
      before,
      current: before,
    }
  }

  if (!applyChanges) {
    return {
      supported: true,
      configured: false,
      applied: false,
      restarted: false,
      before,
      current: before,
    }
  }

  if (before.compatible === false) {
    return {
      supported: false,
      configured: false,
      applied: false,
      restarted: false,
      compatibilityBlocked: true,
      reason: before.compatibilityReason ?? "현재 네트워크 환경에는 패스트핑을 적용하지 않습니다",
      before,
      current: before,
    }
  }

  const current = await runner(true)
  if (!isFastPingConfigured(current)) {
    throw new Error("패스트핑 레지스트리 값을 적용한 뒤 검증에 실패했습니다")
  }

  const restart = restartAfterApply
    ? await restartPrimaryNetworkInterface(current, { restarter })
    : null

  return {
    supported: true,
    configured: true,
    applied: true,
    restarted: restart !== null,
    restart,
    before,
    current,
  }
}

export function printFastPingStatus(result) {
  if (!result.supported) {
    console.log(`패스트핑: 지원하지 않음 (${result.reason})`)
    return
  }

  const status = result.current
  const state = result.configured
    ? (result.restarted ? "적용 및 인터페이스 재시작 완료" : (result.applied ? "적용 완료" : "적용됨"))
    : "적용 안 됨"
  console.log(
    `패스트핑: ${state} | ${status.interfaceAlias} #${status.interfaceIndex}`
      + ` | TcpAckFrequency=${status.TcpAckFrequency ?? "없음"}`
      + `, TCPNoDelay=${status.TCPNoDelay ?? "없음"}`,
  )
}

export function printTcpAutoTuningStatus(result) {
  if (!result.supported) {
    console.log(`TCP 자동 조정: 지원하지 않음 (${result.reason})`)
    return
  }

  const status = result.current
  const state = result.optimized
    ? (result.applied ? "Normal로 복구 완료" : "최적화됨")
    : "비최적화"
  const policy = status.groupPolicy !== "NotConfigured"
    ? ` | 그룹 정책=${status.groupPolicy}`
    : ""
  console.log(`TCP 자동 조정: ${state} | 현재=${status.effective}${policy}`)
}
