
$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new()
$interfaces = @{}
Get-NetIPInterface -AddressFamily IPv4 |
  Where-Object ConnectionState -eq "Connected" |
  ForEach-Object { $interfaces[$_.InterfaceIndex] = $_ }

$route = Get-NetRoute -AddressFamily IPv4 -DestinationPrefix "0.0.0.0/0" -ErrorAction SilentlyContinue |
  Where-Object { $interfaces.ContainsKey($_.InterfaceIndex) } |
  Sort-Object @{ Expression = {
    $_.RouteMetric + $interfaces[$_.InterfaceIndex].InterfaceMetric
  } }, InterfaceIndex |
  Select-Object -First 1

if ($null -eq $route) {
  [pscustomobject]@{
    supported = $false
    disconnected = $true
    reason = "활성 IPv4 기본 경로가 없습니다. 인터넷 연결을 확인한 뒤 다시 시도하세요"
  } | ConvertTo-Json -Compress
  return
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
    Where-Object { $_ -match "(?i)^(insecure_npcap|npcap(?:_wifi)?|selow)$" }
)
$compatibleSecurityBindings = @(
  $thirdPartyBindings |
    Where-Object { $_ -match "(?i)^inca_tkfwfv$" }
)
$compatibleVirtualizationBindings = @(
  $thirdPartyBindings |
    Where-Object { $_ -match "(?i)^vmware_bridge$" }
)
$blockingThirdPartyBindings = @(
  $thirdPartyBindings |
    Where-Object { $_ -notmatch "(?i)^(insecure_npcap|npcap(?:_wifi)?|selow|inca_tkfwfv|vmware_bridge)$" }
)
$knownBlockingBindings = @(
  $blockingThirdPartyBindings |
    Where-Object { $_ -match "(?i)^(nt_rtf64|nt_ndiswgc|nt_ndextlag)$" }
)
$isVirtual = (
  -not [bool]$adapter.HardwareInterface -or
  $adapter.InterfaceDescription -match "(?i)virtual|vpn|tap|tun|wintun|wireguard|hyper-v|vmware|virtualbox"
)
$compatible = -not $isVirtual -and $blockingThirdPartyBindings.Count -eq 0
$compatibilityReason = if ($isVirtual) {
  "VPN 또는 가상 네트워크 어댑터에는 패스트핑을 적용하지 않습니다"
} elseif ($blockingThirdPartyBindings -contains "nt_rtf64") {
  "Realtek 네트워크 가속 필터가 연결되어 있어 안전을 위해 패스트핑을 적용하지 않습니다. 이더넷 속성에서 Realtek LightWeight Filter (NDIS6.40)를 해제한 뒤 다시 시도하세요"
} elseif ($blockingThirdPartyBindings -contains "nt_ndiswgc") {
  "WireSock 또는 WinpkFilter 네트워크 필터가 연결되어 있어 패스트핑을 적용하지 않습니다"
} elseif ($blockingThirdPartyBindings -contains "nt_ndextlag") {
  "ExitLag 네트워크 필터가 연결되어 있어 패스트핑을 적용하지 않습니다"
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
  interfaceDescription = $adapter.InterfaceDescription
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
  compatibleSecurityBindings = $compatibleSecurityBindings
  compatibleVirtualizationBindings = $compatibleVirtualizationBindings
  blockingThirdPartyBindings = $blockingThirdPartyBindings
  knownBlockingBindings = $knownBlockingBindings
  TcpAckFrequency = $ackFrequency
  TCPNoDelay = $noDelay
} | ConvertTo-Json -Compress
