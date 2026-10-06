import assert from 'node:assert/strict'
import test from 'node:test'
import { readFile } from 'node:fs/promises'

const [installer, cleanup, service, rustService, desktopMain, updateInstall] = await Promise.all([
  readFile(new URL('../desktop/installer.nsi', import.meta.url), 'utf8'),
  readFile(new URL('../native/installer-cleanup/src/main.rs', import.meta.url), 'utf8'),
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

test('설치기는 네이티브 helper로 정상 종료와 예약 작업 정리를 수행한다', () => {
  assert.match(cleanup, /installer-close-request/)
  assert.match(cleanup, /SystemTime::now\(\)/)
  assert.match(cleanup, /fn path_is_within/)
  assert.match(cleanup, /fn exclusions\(/)
  assert.match(cleanup, /fn processes_in/)
  assert.match(cleanup, /NogiremUpdater\/updates/)
  assert.match(cleanup, /fn preserved_updater/)
  assert.match(cleanup, /parent_pid/)
  assert.match(installer, /--caller-pid \$9/)
  assert.match(installer, /nsExec::ExecToStack[\s\S]*installer-cleanup\.exe/)
  assert.match(installer, /IfFileExists "\$INSTDIR\\\*\.\*" 0 stop_installed_app_done/)
  assert.doesNotMatch(installer, /powershell|ExecutionPolicy/i)
  assert.match(installer, /\/NOGIREMUPDATE=[\s\S]*--updater-pid \$2/)
  assert.match(updateInstall, /format!\("\/NOGIREMUPDATE=\{\}",std::process::id\(\)\)/)
  assert.match(cleanup, /TerminateProcess/)
  assert.doesNotMatch(cleanup, /powershell|taskkill/i)
  assert.match(rustService, /read_to_string\(&close\)/)
  assert.match(rustService, /next_close_request != close_request/)
  assert.match(cleanup, /Mabinogi Rem Booster Startup/)
  assert.match(cleanup, /GetSystemDirectoryW/)
  assert.match(installer, /UNINSTALL_MANIFEST/)
  assert.doesNotMatch(installer, /RMDir \/r/i)
})

test('업데이트 건강 확인은 UI 열기 뒤 애니메이션 완료 전에 기록한다', () => {
  const start = rustService.match(/pub fn start\(self: &Arc<Self>\) -> Result<\(\)> \{[\s\S]*?\n    \}/)?.[0] ?? ''
  assert.match(start, /open_main\(self\.startup_tray\)\?;[\s\S]*mark_healthy/)
  const animation = rustService.match(/"application:complete-startup-animation"=>[^\n]+/)?.[0] ?? ''
  assert.doesNotMatch(animation, /mark_healthy/)
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
  const paths = [...source.matchAll(/^\s*(?:Delete|RMDir) "\$INSTDIR\\([^"\r\n]+)"$/gm)].map(match => match[1])
  assert.ok(paths.length > 100)
  for (const path of paths) {
    assert.ok(/^(?:runtime|node_modules|src|service)(?:\\|$)|^package\.json$/.test(path), path)
    assert.ok(!path.includes('..') && !path.includes(':') && !path.includes('*') && !path.includes('$'), path)
    assert.ok(!path.endsWith('-preload.js'), path)
  }
})
