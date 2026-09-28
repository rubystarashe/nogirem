param([Parameter(Mandatory=$true)][string]$InstallDirectory, [switch]$Uninstall)
$ErrorActionPreference = 'Stop'
$env:PSModulePath = Join-Path $PSHOME 'Modules'
$instanceDirectory = Join-Path $env:APPDATA '마비노기 렘 부스터\instance'
$primaryFile = Join-Path $instanceDirectory 'primary.json'
$requestFile = Join-Path $instanceDirectory 'installer-close-request'
$targetExe = [IO.Path]::GetFullPath((Join-Path $InstallDirectory 'nogirem.exe'))
$running = @()
if (Test-Path -LiteralPath $primaryFile) {
    try {
        $record = Get-Content -LiteralPath $primaryFile -Raw | ConvertFrom-Json
        $existing = Get-Process -Id $record.pid -ErrorAction SilentlyContinue
        if ($existing -and $existing.Path -eq $targetExe) { $running += $existing }
    } catch { Write-Verbose $_ }
}
$running += @(Get-Process -Name 'nogirem' -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $targetExe })
if ($running.Count -gt 0) {
    New-Item -ItemType Directory -Path $instanceDirectory -Force | Out-Null
    [IO.File]::WriteAllText($requestFile, 'installer-upgrade')
    $deadline = [DateTime]::UtcNow.AddSeconds(90)
    $lastRequest = [DateTime]::UtcNow
    while (@($running | Where-Object { -not $_.HasExited }).Count -gt 0) {
        # Startup can initialize its monitor after our first request. Retry until
        # the same installed processes exit; never kill recording workers.
        if (([DateTime]::UtcNow - $lastRequest).TotalSeconds -ge 1) {
            [IO.File]::WriteAllText($requestFile, 'installer-upgrade')
            $lastRequest = [DateTime]::UtcNow
        }
        if ([DateTime]::UtcNow -ge $deadline) { throw '앱이 종료되지 않았습니다. 녹화 저장을 마치고 앱을 종료한 뒤 다시 시도하세요.' }
        Start-Sleep -Milliseconds 250
    }
}
if ($Uninstall) {
    # Only remove a task belonging to this installation; preserve other installs.
    Get-ScheduledTask | Where-Object { @($_.Actions | Where-Object { $_.Execute.Trim('"') -eq $targetExe }).Count -gt 0 } | Unregister-ScheduledTask -Confirm:$false
}
if (Test-Path -LiteralPath $requestFile) { Remove-Item -LiteralPath $requestFile }
