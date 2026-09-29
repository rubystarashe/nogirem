import assert from 'node:assert/strict'
import test from 'node:test'
import { spawn } from 'node:child_process'
import { copyFile, mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
const [installer, cleanup, service, rustService, desktopMain, updateInstall] = await Promise.all([
  readFile(new URL('../desktop/installer.nsi', import.meta.url), 'utf8'),
  readFile(new URL('../scripts/stop-installed-app.ps1', import.meta.url), 'utf8'),
  readFile(new URL('../service/main.mjs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/backend/src/service.rs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/src/main.rs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/backend/src/update_install.rs', import.meta.url), 'utf8'),
])
test('설치기는 WebView2를 확인하고 설치 EXE를 직접 실행한다', () => {
  assert.match(installer, /SilentInstall silent/)
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
  assert.match(cleanup, /HasExited/)
  assert.match(cleanup, /StartsWith\(\$prefix/)
  assert.match(cleanup, /\$_\.Id -eq \$PID/)
  assert.match(cleanup, /\$_\.Id -eq \$CallerPid/)
  assert.match(installer, /-CallerPid \$9/)
  assert.match(cleanup, /Stop-Process -Force/)
  assert.doesNotMatch(cleanup, /taskkill.*\/IM/i)
  assert.match(rustService, /read_to_string\(&close\)/)
  assert.match(rustService, /next_close_request != close_request/)
  assert.match(cleanup, /\.Execute\.Trim\('"'\) -eq \$targetExe/)
  assert.match(installer, /UNINSTALL_MANIFEST/)
  assert.doesNotMatch(installer, /RMDir \/r/i)
})
test('제거 종료 스크립트는 자신을 실행한 제거기를 종료하지 않는다', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'nogirem-uninstaller-'))
  const caller = join(directory, 'Uninstall.exe')
  await copyFile(process.env.ComSpec, caller)
  const sleeper = spawn(caller, ['/d', '/c', 'ping', '127.0.0.1', '-n', '30'], { windowsHide: true, stdio: 'ignore' })
  try {
    const powershell = join(process.env.SystemRoot, 'System32', 'WindowsPowerShell', 'v1.0', 'powershell.exe')
    const check = spawn(powershell, ['-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass', '-File', fileURLToPath(new URL('../scripts/stop-installed-app.ps1', import.meta.url)), '-InstallDirectory', directory, '-CallerPid', String(sleeper.pid), '-Uninstall'], { windowsHide: true, stdio: 'ignore' })
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
