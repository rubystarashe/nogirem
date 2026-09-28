import assert from 'node:assert/strict'
import test from 'node:test'
import { readFile } from 'node:fs/promises'

const [packager, signer, bridge, workers, updater, launcher, installer] = await Promise.all([
  readFile(new URL('../scripts/package-dioxus.mjs', import.meta.url), 'utf8'),
  readFile(new URL('../scripts/sign-update.mjs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/src/bridge.rs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/backend/src/workers.rs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/backend/src/updater.rs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/portable.nsi', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/backend/src/update_install.rs', import.meta.url), 'utf8'),
])

test('포터블 배포물은 내부 파일을 캐시에 준비하는 단일 EXE다', () => {
  assert.match(packager, /portable\.marker/)
  assert.match(packager, /nogirem-dioxus-portable-\$\{version\}\.exe/)
  assert.match(launcher, /File \/r "\$\{APP_DIRECTORY\}\\\*"/)
  assert.match(launcher, /--portable-source="\$EXEPATH"/)
  assert.match(launcher, /--portable-launcher-pid=\$2/)
  assert.match(launcher, /ExecWait/)
  assert.match(launcher, /NogiremPortableRuntime\\\$\{APP_VERSION\}/)
  assert.match(launcher, /portable-ready\.marker/)
  assert.match(launcher, /IfErrors extraction_failed/)
  assert.match(launcher, /extraction_failed:[\s\S]*RMDir \/r/)
})

test('설치형과 포터블은 별도 서명 업데이트 매니페스트를 사용한다', () => {
  assert.match(signer, /createEnvelope\(installer,'update\.json'\)/)
  assert.match(signer, /createEnvelope\(portable,'portable-update\.json'\)/)
  assert.match(updater, /if portable\{"portable-update\.json"\}else\{"update\.json"\}/)
  assert.match(updater, /nogirem-dioxus-portable-\{\}\.exe/)
  assert.match(installer, /replace_file\(&payload,target\)/)
})

test('포터블도 공유 설정 경로를 사용하고 실행 위치만 업데이트 모드로 전달한다', () => {
  assert.match(workers, /join\("마비노기 렘 부스터"\)/)
  assert.doesNotMatch(bridge, /NOGIREM_USER_DATA/)
  assert.match(bridge, /NOGIREM_PORTABLE_ROOT/)
  assert.match(bridge, /NOGIREM_PORTABLE_SOURCE/)
})
