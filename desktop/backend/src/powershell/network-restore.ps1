
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
