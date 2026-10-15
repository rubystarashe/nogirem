import assert from 'node:assert/strict'
import test from 'node:test'
import { spawn } from 'node:child_process'
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
const [installer, cleanup, retiredCleanup, service, rustService, desktopMain, updateInstall] = await Promise.all([
  readFile(new URL('../desktop/installer.nsi', import.meta.url), 'utf8'),
  readFile(new URL('../scripts/stop-installed-app.ps1', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/retired-assets-cleanup.nsh', import.meta.url), 'utf8'),
  readFile(new URL('../service/main.mjs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/backend/src/service.rs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/src/main.rs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/backend/src/update_install.rs', import.meta.url), 'utf8'),
])
test('설치기는 WebView2를 확인하고 설치 EXE를 직접 실행한다', () => {
  assert.match(installer, /SilentInstall silent/)
  assert.match(installer, /VIProductVersion "\$\{APP_VERSION\}\.0"/)
  assert.match(installer, /VIAddVersionKey \/LANG=1042 "FileDescription"/)
  assert.match(installer, /SetCompressor \/SOLID lzma/)
  assert.match(installer, /Exec '"\$INSTDIR\\nogirem\.exe"'/)
  assert.match(installer, /\$\{GetOptions\} "\$1" "\/S"/)
  assert.match(installer, /F3017226-FE2A-4295-8BDF-00C3A9A7E4C5/)
  assert.match(installer, /\/silent \/install/)
})
test('UI 이전 로그는 보호된 캐시와 고정 panic 단계만 사용한다', () => {
  assert.match(desktopMain, /update_install::open_bootstrap_log/)
  assert.match(desktopMain, /set_hook\(Box::new\(\|_\|bootstrap_log\("PANIC"\)\)\)/)
  assert.doesNotMatch(desktopMain, /PANIC \{info\}/)
  assert.match(updateInstall, /let root=protected_cache_root\(\)\?/)
  assert.match(updateInstall, /lock_safe_directory\(&root\)/)
  assert.match(updateInstall, /FILE_FLAG_OPEN_REPARSE_POINT/)
  assert.match(updateInstall, /GetLastError\(\)\}==ERROR_ALREADY_EXISTS/)
  assert.match(updateInstall, /GetFileInformationByHandle\(guard\.0/)
  assert.match(updateInstall, /require_high_integrity\(&path\)/)
})
test('설치기는 정상 종료를 기다리고 해당 설치의 예약 작업만 제거한다', () => {
  assert.match(cleanup, /installer-close-request/)
  assert.match(cleanup, /Guid\]::NewGuid/)
  assert.match(cleanup, /StartsWith\(\$prefix/)
  assert.match(cleanup, /\$_\.Id -eq \$PID/)
  assert.match(cleanup, /\$_\.Id -eq \$CallerPid/)
  assert.match(cleanup, /function Get-UpdaterProcesses/)
  assert.match(cleanup, /NogiremUpdater\\updates/)
  assert.match(cleanup, /while \(\$true\)[\s\S]*Get-InstalledProcesses/)
  assert.match(cleanup, /Get-CimInstance Win32_Process[\s\S]*ParentProcessId/)
  assert.match(cleanup, /\[int\]\$UpdaterPid = 0/)
  assert.match(installer, /-CallerPid \$9/)
  assert.match(installer, /nsExec::ExecToStack[\s\S]*stop-installed-app\.ps1/)
  assert.match(installer, /IfFileExists "\$INSTDIR\\\*\.\*" 0 stop_installed_app_done/)
  assert.doesNotMatch(installer, /ExecutionPolicy Bypass/)
  assert.match(installer, /\/NOGIREMUPDATE=[\s\S]*-UpdaterPid \$2/)
  assert.match(updateInstall, /format!\("\/NOGIREMUPDATE=\{\}",std::process::id\(\)\)/)
  assert.match(updateInstall, /fn seal_path[\s\S]*set_file_integrity\(path,"H"\)[\s\S]*require_high_integrity\(path\)/)
  assert.match(updateInstall, /fn trusted_state[\s\S]*seal_path\(&dir\.join\("result\.json"\),"result\.json"\)/)
  assert.match(updateInstall, /seal_path\(&backup,"previous-portable\.exe"\)[\s\S]*seal_path\(&recovery,"portable-recovery\.json"\)/)
  assert.match(updateInstall, /pub fn seal_commit[\s\S]*seal_path\(&dir\.join\("commit"\),"commit"\)/)
  assert.match(cleanup, /if \(\$UpdaterPid -gt 0\)[\s\S]*\$parentPid -ne \$UpdaterPid[\s\S]*Test-UpdaterPid \$UpdaterPid[\s\S]*exit 0/)
  assert.match(cleanup, /Stop-Process -Force/)
  assert.doesNotMatch(cleanup, /taskkill.*\/IM/i)
  assert.match(rustService, /read_to_string\(&close\)/)
  assert.match(rustService, /next_close_request != close_request/)
  assert.match(cleanup, /Get-ScheduledTask -TaskPath '\\' -TaskName \$startupTaskName/)
  assert.match(cleanup, /\$task\.TaskName -eq \$startupTaskName/)
  assert.match(cleanup, /\$actions\.Count -eq 1/)
  assert.match(cleanup, /GetFullPath\(\$actions\[0\]\.Execute\.Trim\('"'\)\)\.Equals\(\$targetExe/)
  assert.match(cleanup, /\$actions\[0\]\.Arguments -ceq '--startup-tray'/)
  assert.match(cleanup, /Unregister-ScheduledTask[\s\S]*catch \{[\s\S]*앱 제거는 계속합니다/)
  assert.match(installer, /UNINSTALL_MANIFEST/)
  assert.equal([...installer.matchAll(/!include "retired-assets-cleanup\.nsh"/g)].length, 2)
  assert.match(installer, /DetailPrint "\$1"[\s\S]*MessageBox MB_ICONSTOP[\s\S]*\$1/)
  assert.doesNotMatch(installer, /RMDir \/r/i)
})
test('이전 설치형의 제거된 앱 파일만 정확히 정리한다', () => {
  const expected = [
    'TURBO_KEY_TERMS.md',
    'overlay-settings.html',
    'debuff-region-overlay.html',
    'debuff-alert-overlay.html',
    'overlay-configuration.html',
    'debuff.csv',
    'service\\overlay-settings-preload.js',
    'service\\debuff-region-overlay-preload.js',
    'service\\debuff-alert-overlay-preload.js',
    'service\\overlay-configuration-preload.js',
    'public\\main3.jpg',
    'public\\doc_minimize\\introduce.png',
  ]
  const deleted = [...retiredCleanup.matchAll(/^\s*Delete "\$INSTDIR\\([^"\r\n]+)"$/gm)].map(match => match[1])
  assert.deepEqual(deleted, expected)
  assert.doesNotMatch(retiredCleanup, /(?:Delete|RMDir)\s+\/r|\*/)
  assert.doesNotMatch(retiredCleanup, /\$APPDATA|\$LOCALAPPDATA|\.\.|:/)
})
test('업데이트 건강 확인은 UI 열기 뒤 애니메이션 완료 전에 기록한다', () => {
  const start = rustService.match(/pub fn start\(self: &Arc<Self>\) -> Result<\(\)> \{[\s\S]*?\n    \}/)?.[0] ?? ''
  assert.match(start, /open_main\(self\.startup_tray\)\?;[\s\S]*mark_healthy/)
  const animation = rustService.match(/"application:complete-startup-animation"=>[^\n]+/)?.[0] ?? ''
  assert.doesNotMatch(animation, /mark_healthy/)
})
test('제거 종료 스크립트는 자신을 실행한 제거기를 종료하지 않는다', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'nogirem-uninstaller-'))
  const caller = join(directory, 'Uninstall.exe')
  await copyFile(process.env.ComSpec, caller)
  const sleeper = spawn(caller, ['/d', '/c', 'ping', '127.0.0.1', '-n', '30'], { windowsHide: true, stdio: 'ignore' })
  try {
    const powershell = join(process.env.SystemRoot, 'System32', 'WindowsPowerShell', 'v1.0', 'powershell.exe')
    const environment = {
      ...process.env,
      APPDATA: join(directory, 'roaming'),
      LOCALAPPDATA: join(directory, 'local'),
    }
    const check = spawn(powershell, ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', fileURLToPath(new URL('../scripts/stop-installed-app.ps1', import.meta.url)), '-InstallDirectory', directory, '-CallerPid', String(sleeper.pid), '-Uninstall'], { windowsHide: true, stdio: 'ignore', env: environment })
    const code = await Promise.race([
      new Promise(resolve => check.once('exit', resolve)),
      new Promise((_, reject) => setTimeout(() => reject(new Error('종료 스크립트 시간 초과')), 8000)),
    ])
    assert.equal(code, 0)
    assert.equal(sleeper.exitCode, null)
  } finally {
    if (sleeper.exitCode === null) {
      sleeper.kill()
      await new Promise(resolve => sleeper.once('exit', resolve))
    }
    await rm(directory, { recursive: true, force: true })
  }
})
test('종료 스크립트는 updater와 대기 중 새로 생성된 설치 프로세스를 종료한다', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'nogirem-rescan-'))
  const local = join(directory, 'local')
  const install = join(directory, 'install')
  const updaterDirectory = join(local, 'NogiremUpdater', 'updates', 'job-test')
  await mkdir(install, { recursive: true })
  await mkdir(updaterDirectory, { recursive: true })
  const app = join(install, 'nogirem.exe')
  const updater = join(updaterDirectory, 'updater.exe')
  await copyFile(process.env.ComSpec, app)
  await copyFile(process.env.ComSpec, updater)
  const children = [
    spawn(app, ['/d', '/c', 'ping', '127.0.0.1', '-n', '30'], { windowsHide: true, stdio: 'ignore' }),
    spawn(updater, ['/d', '/c', 'ping', '127.0.0.1', '-n', '30'], { windowsHide: true, stdio: 'ignore' }),
  ]
  try {
    const powershell = join(process.env.SystemRoot, 'System32', 'WindowsPowerShell', 'v1.0', 'powershell.exe')
    const environment = {
      ...process.env,
      APPDATA: join(directory, 'roaming'),
      LOCALAPPDATA: local,
    }
    const check = spawn(powershell, [
      '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass',
      '-File', fileURLToPath(new URL('../scripts/stop-installed-app.ps1', import.meta.url)),
      '-InstallDirectory', install,
      '-CallerPid', String(process.pid),
      '-GracefulTimeoutSeconds', '1',
    ], { windowsHide: true, stdio: 'ignore', env: environment })
    await new Promise(resolve => setTimeout(resolve, 300))
    children.push(spawn(app, ['/d', '/c', 'ping', '127.0.0.1', '-n', '30'], { windowsHide: true, stdio: 'ignore' }))
    const code = await Promise.race([
      new Promise(resolve => check.once('exit', resolve)),
      new Promise((_, reject) => setTimeout(() => reject(new Error('재탐색 종료 스크립트 시간 초과')), 8000)),
    ])
    assert.equal(code, 0)
    for (const child of children) assert.notEqual(child.exitCode, null)
  } finally {
    for (const child of children) {
      if (child.exitCode === null) {
        child.kill()
        await new Promise(resolve => child.once('exit', resolve))
      }
    }
    await rm(directory, { recursive: true, force: true })
  }
})
test('검증된 updater 설치는 이미 종료를 마친 앱을 다시 탐색하지 않는다', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'nogirem-trusted-update-'))
  const local = join(directory, 'local')
  const install = join(directory, 'install')
  const updaterDirectory = join(local, 'NogiremUpdater', 'updates', 'job-test')
  const unrelatedUpdaterDirectory = join(local, 'NogiremUpdater', 'updates', 'job-other')
  await mkdir(install, { recursive: true })
  await mkdir(updaterDirectory, { recursive: true })
  await mkdir(unrelatedUpdaterDirectory, { recursive: true })
  const app = join(install, 'nogirem.exe')
  const updater = join(updaterDirectory, 'updater.exe')
  const unrelatedUpdater = join(unrelatedUpdaterDirectory, 'updater.exe')
  const installer = join(updaterDirectory, 'installer.exe')
  const helper = join(directory, 'updater-helper.mjs')
  const state = join(directory, 'child.json')
  await copyFile(process.env.ComSpec, app)
  await copyFile(process.execPath, updater)
  await copyFile(process.execPath, unrelatedUpdater)
  await copyFile(process.env.ComSpec, installer)
  await writeFile(helper, [
    'import { spawn } from "node:child_process"',
    'import { writeFileSync } from "node:fs"',
    'const child = spawn(process.argv[2], ["/d", "/c", "ping", "127.0.0.1", "-n", "30"], { stdio: "ignore", windowsHide: true })',
    'writeFileSync(process.argv[3], JSON.stringify({ pid: child.pid }))',
    'child.once("exit", code => process.exit(code ?? 0))',
    'setInterval(() => {}, 1000)',
  ].join('\n'))
  const installedProcess = spawn(app, ['/d', '/c', 'ping', '127.0.0.1', '-n', '30'], { windowsHide: true, stdio: 'ignore' })
  const updaterProcess = spawn(updater, [helper, installer, state], { windowsHide: true, stdio: 'ignore' })
  const unrelatedUpdaterProcess = spawn(unrelatedUpdater, [helper, installer, join(directory, 'unrelated-child.json')], { windowsHide: true, stdio: 'ignore' })
  let installerPid = 0
  try {
    const deadline = Date.now() + 5000
    while (!installerPid && Date.now() < deadline) {
      try {
        installerPid = JSON.parse(await readFile(state, 'utf8')).pid
      } catch {
        await new Promise(resolve => setTimeout(resolve, 50))
      }
    }
    assert.ok(installerPid > 0)
    const powershell = join(process.env.SystemRoot, 'System32', 'WindowsPowerShell', 'v1.0', 'powershell.exe')
    const environment = {
      ...process.env,
      APPDATA: join(directory, 'roaming'),
      LOCALAPPDATA: local,
    }
    const forged = spawn(powershell, [
      '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass',
      '-File', fileURLToPath(new URL('../scripts/stop-installed-app.ps1', import.meta.url)),
      '-InstallDirectory', install,
      '-CallerPid', String(installerPid),
      '-UpdaterPid', String(unrelatedUpdaterProcess.pid),
      '-GracefulTimeoutSeconds', '1',
    ], { windowsHide: true, stdio: 'ignore', env: environment })
    const forgedCode = await Promise.race([
      new Promise(resolve => forged.once('exit', resolve)),
      new Promise((_, reject) => setTimeout(() => reject(new Error('위조 updater PID 검사 시간 초과')), 8000)),
    ])
    assert.notEqual(forgedCode, 0)
    const check = spawn(powershell, [
      '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass',
      '-File', fileURLToPath(new URL('../scripts/stop-installed-app.ps1', import.meta.url)),
      '-InstallDirectory', install,
      '-CallerPid', String(installerPid),
      '-UpdaterPid', String(updaterProcess.pid),
      '-GracefulTimeoutSeconds', '1',
    ], { windowsHide: true, stdio: 'ignore', env: environment })
    const code = await Promise.race([
      new Promise(resolve => check.once('exit', resolve)),
      new Promise((_, reject) => setTimeout(() => reject(new Error('검증된 updater 종료 스크립트 시간 초과')), 8000)),
    ])
    assert.equal(code, 0)
    assert.equal(installedProcess.exitCode, null)
    assert.equal(updaterProcess.exitCode, null)
  } finally {
    for (const child of [installedProcess, updaterProcess, unrelatedUpdaterProcess]) {
      if (child.exitCode === null) {
        child.kill()
        await new Promise(resolve => child.once('exit', resolve))
      }
    }
    await rm(directory, { recursive: true, force: true })
  }
})
test('0.4.1 updater가 인자 없이 실행한 새 설치기는 검증된 부모 helper를 보존한다', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'nogirem-legacy-updater-'))
  const local = join(directory, 'local')
  const install = join(directory, 'install')
  const updaterDirectory = join(local, 'NogiremUpdater', 'updates', 'job-test')
  await mkdir(install, { recursive: true })
  await mkdir(updaterDirectory, { recursive: true })
  const updater = join(updaterDirectory, 'updater.exe')
  const installer = join(updaterDirectory, 'installer.exe')
  const helper = join(directory, 'legacy-updater.mjs')
  const state = join(directory, 'child.json')
  await copyFile(process.execPath, updater)
  await copyFile(process.env.ComSpec, installer)
  await writeFile(helper, [
    'import { spawn } from "node:child_process"',
    'import { writeFileSync } from "node:fs"',
    'const child = spawn(process.argv[2], ["/d", "/c", "ping", "127.0.0.1", "-n", "30"], { stdio: "ignore", windowsHide: true })',
    'writeFileSync(process.argv[3], JSON.stringify({ pid: child.pid }))',
    'child.once("exit", code => process.exit(code ?? 0))',
    'setInterval(() => {}, 1000)',
  ].join('\n'))
  const legacyUpdater = spawn(updater, [helper, installer, state], { windowsHide: true, stdio: 'ignore' })
  let installerPid = 0
  try {
    const deadline = Date.now() + 5000
    while (!installerPid && Date.now() < deadline) {
      try {
        installerPid = JSON.parse(await readFile(state, 'utf8')).pid
      } catch {
        await new Promise(resolve => setTimeout(resolve, 50))
      }
    }
    assert.ok(installerPid > 0)
    const powershell = join(process.env.SystemRoot, 'System32', 'WindowsPowerShell', 'v1.0', 'powershell.exe')
    const environment = {
      ...process.env,
      APPDATA: join(directory, 'roaming'),
      LOCALAPPDATA: local,
    }
    const check = spawn(powershell, [
      '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass',
      '-File', fileURLToPath(new URL('../scripts/stop-installed-app.ps1', import.meta.url)),
      '-InstallDirectory', install,
      '-CallerPid', String(installerPid),
      '-GracefulTimeoutSeconds', '1',
    ], { windowsHide: true, stdio: 'ignore', env: environment })
    const code = await Promise.race([
      new Promise(resolve => check.once('exit', resolve)),
      new Promise((_, reject) => setTimeout(() => reject(new Error('구버전 updater 호환 검사 시간 초과')), 8000)),
    ])
    assert.equal(code, 0)
    assert.equal(legacyUpdater.exitCode, null)
    assert.doesNotThrow(() => process.kill(installerPid, 0))
  } finally {
    if (installerPid > 0) {
      try { process.kill(installerPid) } catch {}
    }
    if (legacyUpdater.exitCode === null) {
      await Promise.race([
        new Promise(resolve => legacyUpdater.once('exit', resolve)),
        new Promise(resolve => setTimeout(resolve, 2000)),
      ])
    }
    if (legacyUpdater.exitCode === null) {
      legacyUpdater.kill()
      await new Promise(resolve => legacyUpdater.once('exit', resolve))
    }
    await rm(directory, { recursive: true, force: true })
  }
})
test('앱은 보존한 시작 설정으로 새 실행 파일의 예약 작업을 복구한다', () => {
  assert.match(service, /async function getStartupTraySetting\(\)[\s\S]*state\.preferred \|\| valid[\s\S]*applyStartupTraySetting\(true\)/)
  assert.match(service, /New-ScheduledTaskAction -Execute \$\{quotePowerShellLiteral\(app\.getPath\("exe"\)\)\}/)
})


test('Node 업그레이드 정리는 구 패키지의 명시된 파일만 삭제한다', async () => {
  const source = await readFile(new URL('../desktop/legacy-node-cleanup.nsh', import.meta.url), 'utf8')
  assert.match(source, /FileExists.*runtime\\node.exe/)
  assert.match(source, /FileExists.*service\\main.mjs/)
  assert.doesNotMatch(source, /RMDir \/r/i)
  const paths = [...source.matchAll(/^\s*(?:Delete|RMDir) "\$INSTDIR\\([^"\r\n]+)"$/gm)].map(m => m[1])
  assert.ok(paths.length > 100)
  for (const path of paths) {
    assert.ok(/^(?:runtime|node_modules|src|service)(?:\\|$)|^package\.json$/.test(path), path)
    assert.ok(!path.includes('..') && !path.includes(':') && !path.includes('*') && !path.includes('$'), path)
    assert.ok(!path.endsWith('-preload.js'), path)
  }
})
