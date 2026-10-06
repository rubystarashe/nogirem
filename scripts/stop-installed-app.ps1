param(
    [Parameter(Mandatory=$true)][string]$InstallDirectory,
    [Parameter(Mandatory=$true)][int]$CallerPid,
    [switch]$Uninstall,
    [int]$UpdaterPid = 0,
    [ValidateRange(1, 300)][int]$GracefulTimeoutSeconds = 30
)
$ErrorActionPreference = 'Stop'
$env:PSModulePath = Join-Path $PSHOME 'Modules'
$instanceDirectory = Join-Path $env:APPDATA '마비노기 렘 부스터\instance'
$requestFile = Join-Path $instanceDirectory 'installer-close-request'
$targetExe = [IO.Path]::GetFullPath((Join-Path $InstallDirectory 'nogirem.exe'))
$updaterDirectory = Join-Path $env:LOCALAPPDATA 'NogiremUpdater\updates'
$updaterPrefix = [IO.Path]::GetFullPath($updaterDirectory).TrimEnd('\') + '\'
function Get-InstalledProcesses {
    $prefix = [IO.Path]::GetFullPath($InstallDirectory).TrimEnd('\') + '\'
    @(Get-Process -ErrorAction SilentlyContinue | Where-Object {
        if ($_.Id -eq $PID -or $_.Id -eq $CallerPid) { return $false }
        try {
            $path = $_.Path
            $path -and ([IO.Path]::GetFullPath($path).StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase))
        } catch { $false }
    })
}
function Get-UpdaterProcesses {
    @(Get-Process -ErrorAction SilentlyContinue | Where-Object {
        if ($_.Id -eq $PID -or $_.Id -eq $CallerPid -or $_.Id -eq $script:preservedUpdaterPid) { return $false }
        try {
            $path = $_.Path
            $path -and ([IO.Path]::GetFullPath($path).StartsWith($updaterPrefix, [StringComparison]::OrdinalIgnoreCase))
        } catch { $false }
    })
}
function Test-UpdaterPid([int]$ProcessId) {
    if ($ProcessId -le 0) { return $false }
    try {
        $process = Get-Process -Id $ProcessId -ErrorAction Stop
        $path = [IO.Path]::GetFullPath($process.Path)
        $path.StartsWith($updaterPrefix, [StringComparison]::OrdinalIgnoreCase) -and
            [IO.Path]::GetFileName($path).Equals('updater.exe', [StringComparison]::OrdinalIgnoreCase)
    } catch { $false }
}
$preservedUpdaterPid = 0
if (Test-UpdaterPid $UpdaterPid) {
    $preservedUpdaterPid = $UpdaterPid
} else {
    # 0.4.1 updater는 전용 인자를 전달하지 않으므로 검증된 설치기 부모만 자동 보존한다.
    try {
        $caller = Get-CimInstance Win32_Process -Filter "ProcessId = $CallerPid" -ErrorAction Stop
        $parentPid = [int]$caller.ParentProcessId
        if (Test-UpdaterPid $parentPid) { $preservedUpdaterPid = $parentPid }
    } catch { Write-Verbose $_ }
}
if ($Uninstall) {
    # 제거가 중단돼도 재부팅 때 같은 앱이 다시 시작되지 않도록 대상 예약 작업을 먼저 해제한다.
    Get-ScheduledTask | Where-Object { @($_.Actions | Where-Object { $_.Execute -and $_.Execute.Trim('"') -eq $targetExe }).Count -gt 0 } | Unregister-ScheduledTask -Confirm:$false
}
$updaters = Get-UpdaterProcesses
if ($updaters.Count -gt 0) {
    $updaters | Stop-Process -Force -ErrorAction SilentlyContinue
    $updaterDeadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        Start-Sleep -Milliseconds 100
        $updaters = Get-UpdaterProcesses
    } while ($updaters.Count -gt 0 -and [DateTime]::UtcNow -lt $updaterDeadline)
    if ($updaters.Count -gt 0) {
        throw '업데이트 설치 프로세스를 종료하지 못했습니다. 작업 관리자에서 앱 업데이트를 종료한 뒤 다시 시도하세요.'
    }
}
$deadline = [DateTime]::UtcNow.AddSeconds($GracefulTimeoutSeconds)
$forceDeadline = $null
$quietSince = $null
$lastRequest = [DateTime]::MinValue
New-Item -ItemType Directory -Path $instanceDirectory -Force | Out-Null
while ($true) {
    # 종료 대기 중 새로 시작된 앱과 helper도 매 반복마다 다시 찾는다.
    $running = @(Get-InstalledProcesses | Sort-Object Id -Unique)
    if ($running.Count -eq 0) {
        if ($null -eq $quietSince) { $quietSince = [DateTime]::UtcNow }
        if (([DateTime]::UtcNow - $quietSince).TotalSeconds -ge 1) { break }
    } else {
        $quietSince = $null
        if (([DateTime]::UtcNow - $lastRequest).TotalSeconds -ge 1) {
            [IO.File]::WriteAllText($requestFile, ('installer-upgrade:' + [Guid]::NewGuid().ToString('N')))
            $lastRequest = [DateTime]::UtcNow
        }
        if ([DateTime]::UtcNow -ge $deadline) {
            # 정상 종료가 고착되면 현재 설치기·제거기를 제외한 소유 프로세스만 종료한다.
            $running | Stop-Process -Force -ErrorAction SilentlyContinue
            if ($null -eq $forceDeadline) { $forceDeadline = [DateTime]::UtcNow.AddSeconds(5) }
            if ([DateTime]::UtcNow -ge $forceDeadline -and (Get-InstalledProcesses).Count -gt 0) {
                throw '설치 폴더의 앱 프로세스를 종료하지 못했습니다. 작업 관리자에서 앱을 종료한 뒤 다시 시도하세요.'
            }
        }
    }
    Start-Sleep -Milliseconds 100
}
if (Test-Path -LiteralPath $requestFile) { Remove-Item -LiteralPath $requestFile }
