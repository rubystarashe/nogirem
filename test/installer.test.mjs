import assert from 'node:assert/strict'
import test from 'node:test'
import { readFile } from 'node:fs/promises'
const [installer, cleanup, service] = await Promise.all([
  readFile(new URL('../desktop/installer.nsi', import.meta.url), 'utf8'),
  readFile(new URL('../scripts/stop-installed-app.ps1', import.meta.url), 'utf8'),
  readFile(new URL('../service/main.mjs', import.meta.url), 'utf8'),
])
test('설치기는 WebView2를 확인하고 설치 EXE를 직접 실행한다', () => {
  assert.match(installer, /MUI_FINISHPAGE_RUN "\$INSTDIR\\nogirem.exe"/)
  assert.match(installer, /F3017226-FE2A-4295-8BDF-00C3A9A7E4C5/)
  assert.match(installer, /\/silent \/install/)
})
test('설치기는 정상 종료를 기다리고 해당 설치의 예약 작업만 제거한다', () => {
  assert.match(cleanup, /installer-close-request/)
  assert.match(cleanup, /HasExited/)
  assert.doesNotMatch(cleanup, /Stop-Process|taskkill/i)
  assert.match(cleanup, /\.Execute\.Trim\('"'\) -eq \$targetExe/)
  assert.match(installer, /UNINSTALL_MANIFEST/)
  assert.doesNotMatch(installer, /RMDir \/r/i)
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
