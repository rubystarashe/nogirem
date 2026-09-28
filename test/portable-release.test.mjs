import assert from 'node:assert/strict'
import test from 'node:test'
import { readFile } from 'node:fs/promises'

const [packager, signer, bridge, workers, updater] = await Promise.all([
  readFile(new URL('../scripts/package-dioxus.mjs', import.meta.url), 'utf8'),
  readFile(new URL('../scripts/sign-update.mjs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/src/bridge.rs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/backend/src/workers.rs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/backend/src/updater.rs', import.meta.url), 'utf8'),
])

test('포터블 ZIP은 모드 표시와 소유 파일 매니페스트를 포함한다', () => {
  assert.match(packager, /portable\.marker/)
  assert.match(packager, /portable-manifest\.json/)
  assert.match(packager, /nogirem-dioxus-portable-\$\{version\}\.zip/)
})

test('설치형과 포터블은 별도 서명 업데이트 매니페스트를 사용한다', () => {
  assert.match(signer, /createEnvelope\(installer,'update\.json'\)/)
  assert.match(signer, /createEnvelope\(portable,'portable-update\.json'\)/)
  assert.match(updater, /if portable\{"portable-update\.json"\}else\{"update\.json"\}/)
})

test('포터블도 공유 설정 경로를 사용하고 실행 위치만 업데이트 모드로 전달한다', () => {
  assert.match(workers, /join\("마비노기 렘 부스터"\)/)
  assert.doesNotMatch(bridge, /NOGIREM_USER_DATA/)
  assert.match(bridge, /NOGIREM_PORTABLE_ROOT/)
})
