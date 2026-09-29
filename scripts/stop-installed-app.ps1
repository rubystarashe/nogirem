param([Parameter(Mandatory=$true)][string]$InstallDirectory, [switch]$Uninstall)
$ErrorActionPreference = 'Stop'
$env:PSModulePath = Join-Path $PSHOME 'Modules'
$instanceDirectory = Join-Path $env:APPDATA '마비노기 렘 부스터\instance'
$primaryFile = Join-Path $instanceDirectory 'primary.json'
$requestFile = Join-Path $instanceDirectory 'installer-close-request'
$targetExe = [IO.Path]::GetFullPath((Join-Path $InstallDirectory 'nogirem.exe'))
function Get-InstalledProcesses {
    $prefix = [IO.Path]::GetFullPath($InstallDirectory).TrimEnd('\') + '\'
    @(Get-Process -ErrorAction SilentlyContinue | Where-Object {
        if ($_.Id -eq $PID) { return $false }
        try {
            $path = $_.Path
            $path -and ([IO.Path]::GetFullPath($path).StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase))
        } catch { $false }
    })
}
$running = @()
if (Test-Path -LiteralPath $primaryFile) {
    try {
        $record = Get-Content -LiteralPath $primaryFile -Raw | ConvertFrom-Json
        $existing = Get-Process -Id $record.pid -ErrorAction SilentlyContinue
        if ($existing -and $existing.Path -eq $targetExe) { $running += $existing }
    } catch { Write-Verbose $_ }
}
$running += Get-InstalledProcesses
$running = @($running | Sort-Object Id -Unique)
if ($running.Count -gt 0) {
    New-Item -ItemType Directory -Path $instanceDirectory -Force | Out-Null
    [IO.File]::WriteAllText($requestFile, ('installer-upgrade:' + [Guid]::NewGuid().ToString('N')))
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    $lastRequest = [DateTime]::UtcNow
    while (@($running | Where-Object { -not $_.HasExited }).Count -gt 0) {
        # 시작 감시기가 첫 요청 뒤 준비될 수 있으므로 같은 설치 프로세스가 종료될 때까지 요청을 갱신한다.
        if (([DateTime]::UtcNow - $lastRequest).TotalSeconds -ge 1) {
            [IO.File]::WriteAllText($requestFile, ('installer-upgrade:' + [Guid]::NewGuid().ToString('N')))
            $lastRequest = [DateTime]::UtcNow
        }
        if ([DateTime]::UtcNow -ge $deadline) {
            # 정상 종료가 고착되면 현재 제거기를 제외한 설치 폴더 내부 프로세스만 종료한다.
            $installed = Get-InstalledProcesses
            if ($installed.Count -gt 0) {
                $installed | Stop-Process -Force -ErrorAction SilentlyContinue
                $forceDeadline = [DateTime]::UtcNow.AddSeconds(5)
                do {
                    Start-Sleep -Milliseconds 100
                    $installed = Get-InstalledProcesses
                } while ($installed.Count -gt 0 -and [DateTime]::UtcNow -lt $forceDeadline)
            }
            if ($installed.Count -gt 0) { throw '설치 폴더의 앱 프로세스를 종료하지 못했습니다. 작업 관리자에서 앱을 종료한 뒤 다시 시도하세요.' }
            break
        }
        Start-Sleep -Milliseconds 250
    }
}
if ($Uninstall) {
    # 이 설치 경로의 예약 작업만 제거하고 다른 설치는 보존한다.
    Get-ScheduledTask | Where-Object { @($_.Actions | Where-Object { $_.Execute.Trim('"') -eq $targetExe }).Count -gt 0 } | Unregister-ScheduledTask -Confirm:$false
}
if (Test-Path -LiteralPath $requestFile) { Remove-Item -LiteralPath $requestFile }
