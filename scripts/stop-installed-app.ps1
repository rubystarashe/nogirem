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
$parentPid = 0
try {
    $caller = Get-CimInstance Win32_Process -Filter "ProcessId = $CallerPid" -ErrorAction Stop
    $parentPid = [int]$caller.ParentProcessId
} catch { Write-Verbose $_ }
if ($UpdaterPid -gt 0) {
    if ($parentPid -ne $UpdaterPid -or !(Test-UpdaterPid $UpdaterPid)) {
        throw '업데이트 설치 프로세스의 부모와 경로를 확인하지 못했습니다.'
    }
    $preservedUpdaterPid = $UpdaterPid
} elseif (Test-UpdaterPid $parentPid) {
    # 0.4.1 updater는 전용 인자를 전달하지 않으므로 검증된 설치기 부모만 자동 보존한다.
    $preservedUpdaterPid = $parentPid
}
if ($Uninstall) {
    # 예약 작업 정리 실패는 앱 프로세스 종료 실패로 오인하지 않고 제거를 계속한다.
    try {
        $startupTaskName = 'Mabinogi Rem Booster Startup'
        $startupTasks = @(Get-ScheduledTask -TaskPath '\' -TaskName $startupTaskName -ErrorAction SilentlyContinue)
        foreach ($task in $startupTasks) {
            $actions = @($task.Actions)
            if ($task.TaskPath -eq '\' -and
                $task.TaskName -eq $startupTaskName -and
                $actions.Count -eq 1 -and
                $actions[0].Execute -and
                [IO.Path]::GetFullPath($actions[0].Execute.Trim('"')).Equals($targetExe, [StringComparison]::OrdinalIgnoreCase) -and
                $actions[0].Arguments -ceq '--startup-tray') {
                Unregister-ScheduledTask -InputObject $task -Confirm:$false -ErrorAction Stop
            }
        }
    } catch {
        [Console]::Error.WriteLine("시작 예약 작업을 정리하지 못했지만 앱 제거는 계속합니다: $($_.Exception.Message)")
    }
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
if ($UpdaterPid -gt 0) {
    # 서명 payload를 적용하는 helper는 기록된 앱·backend PID 종료와 worker flush를 이미 완료했다.
    # 설치기에서 설치 폴더를 다시 탐색하면 종료 중인 보조 프로세스를 일반 앱으로 오판할 수 있다.
    exit 0
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
