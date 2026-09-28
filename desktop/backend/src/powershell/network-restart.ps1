
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
