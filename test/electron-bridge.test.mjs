import assert from 'node:assert/strict'
import test from 'node:test'
import {readFile} from 'node:fs/promises'

const [bridge,release,releaseAssets,routing,installer]=await Promise.all([
  readFile(new URL('../scripts/prepare-electron-bridge.mjs',import.meta.url),'utf8'),
  readFile(new URL('../scripts/prepare-release.mjs',import.meta.url),'utf8'),
  readFile(new URL('../scripts/release-assets.mjs',import.meta.url),'utf8'),
  readFile(new URL('../scripts/test-release-routing.mjs',import.meta.url),'utf8'),
  readFile(new URL('../desktop/backend/src/update_install.rs',import.meta.url),'utf8'),
])

test('0.3.19 전환본은 기존 업데이트 UI에 Rust 상태와 재시도를 연결한다',()=>{
  assert.match(bridge,/bridgeVersion='0\.3\.19'/)
  assert.match(bridge,/registerMigration\(setApplicationUpdateState\)/)
  assert.match(bridge,/requestMigration\(\)/)
  assert.match(bridge,/--migration-attempt=/)
  assert.match(bridge,/migration-status\.json/)
  assert.match(bridge,/phase:'error'/)
  assert.match(bridge,/다시 시도/)
  assert.match(bridge,/applicationUpdateDismissed/)
  assert.match(bridge,/onDismiss/)
  assert.match(bridge,/숨기기/)
  assert.match(bridge,/Rust 전환 helper 오류/)
  assert.match(bridge,/await refresh\(attempt\)/)
  assert.match(bridge,/createMigrationAttemptGate/)
  assert.match(bridge,/attempts\.stop\(attempt\)/)
  assert.match(bridge,/60_000/)
  assert.match(bridge,/!handedOff/)
  assert.match(bridge,/NOGIREM_BRIDGE_TARGET/)
  assert.match(bridge,/still contains image-name process termination/)
  assert.match(installer,/let mut apply=start/)
  assert.match(installer,/helper-ready/)
  assert.match(installer,/apply\.try_wait\(\)/)
  assert.match(installer,/terminate\(&mut apply\)/)
})

test('릴리스 조립과 라우팅 계약은 0.3.19 전환본을 요구한다',()=>{
  assert.match(bridge,/nogirem-legacy-electron-migration-\$\{version\}\.\$\{ext\}/)
  assert.match(release,/assembleReleaseAssets/)
  assert.match(releaseAssets,/nogirem-legacy-electron-migration-0\.3\.19\.exe/)
  assert.match(releaseAssets,/nogirem-setup-\$\{version\}\.exe/)
  assert.match(releaseAssets,/nogirem-portable-\$\{version\}\.exe/)
  assert.match(releaseAssets,/oldLegacy:'nogirem-setup-0\.3\.19\.exe'/)
  assert.match(releaseAssets,/version:\\s\*0\\\.3\\\.19/)
  assert.match(releaseAssets,/Electron migration blockmap is required/)
  assert.match(routing,/info\.version,'0\.3\.19'/)
  assert.match(routing,/nogirem-legacy-electron-migration-0\.3\.19\.exe/)
})
