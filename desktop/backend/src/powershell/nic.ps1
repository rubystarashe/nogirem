
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

$route = Get-NetRoute -AddressFamily IPv4 -DestinationPrefix "0.0.0.0/0" -ErrorAction SilentlyContinue |
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
