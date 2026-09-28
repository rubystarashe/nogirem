
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
