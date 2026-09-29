import assert from 'node:assert/strict'
import test from 'node:test'
import { readFile } from 'node:fs/promises'

const [packager, signer, bridge, workers, updater, launcher, installer, installerScript, electronBridgeBuilder] = await Promise.all([
  readFile(new URL('../scripts/package-dioxus.mjs', import.meta.url), 'utf8'),
  readFile(new URL('../scripts/sign-update.mjs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/src/bridge.rs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/backend/src/workers.rs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/backend/src/updater.rs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/portable.nsi', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/backend/src/update_install.rs', import.meta.url), 'utf8'),
  readFile(new URL('../desktop/installer.nsi', import.meta.url), 'utf8'),
  readFile(new URL('../scripts/prepare-electron-bridge.mjs', import.meta.url), 'utf8'),
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
  assert.match(packager, /const portableCacheId = \(await hashDirectory\(portableDirectory\)\)\.digest\('hex'\)/)
  assert.match(packager, /\/DPORTABLE_CACHE_ID=\$\{portableCacheId\}/)
  assert.match(launcher, /StrCmp \$7 "\$\{PORTABLE_CACHE_ID\}" cache_ready/)
  assert.match(launcher, /cache_in_use:/)
  assert.match(launcher, /\$\$found\.Count -gt 0\) \{ exit 2 \}/)
  assert.match(launcher, /StrCmp \$8 2 cache_in_use/)
  assert.doesNotMatch(launcher, /exit 1/)
  assert.doesNotMatch(electronBridgeBuilder, /exit 1/)
  assert.match(launcher, /IfErrors extraction_failed/)
  assert.match(launcher, /extraction_failed:[\s\S]*RMDir \/r/)
})

test('설치형과 포터블은 별도 서명 업데이트 매니페스트를 사용한다', () => {
  assert.match(signer, /createEnvelope\(installer,'update\.json'\)/)
  assert.match(signer, /createEnvelope\(portable,'portable-update\.json'\)/)
  assert.match(updater, /if portable\{"portable-update\.json"\}else\{"update\.json"\}/)
  assert.match(updater, /nogirem-dioxus-portable-\{\}\.exe/)
  assert.match(installer, /replace_file\(&payload,target,&manifest\.sha256\)/)
  assert.match(installerScript, /SilentInstall silent/)
  assert.match(installerScript, /\$\{GetOptions\} "\$1" "\/S"/)
  assert.match(installerScript, /Exec '"\$INSTDIR\\nogirem\.exe"'/)
})

test('포터블도 공유 설정 경로를 사용하고 실행 위치만 업데이트 모드로 전달한다', () => {
  assert.match(workers, /join\("마비노기 렘 부스터"\)/)
  assert.doesNotMatch(bridge, /NOGIREM_USER_DATA/)
  assert.match(bridge, /NOGIREM_PORTABLE_ROOT/)
  assert.match(bridge, /NOGIREM_PORTABLE_SOURCE/)
})

test('포터블 교체와 복구는 보호된 작업 및 실행 파일을 검증한다', () => {
  assert.match(installer, /fn trusted_job\(/)
  assert.match(installer, /require_high_integrity\(&temporary\)/)
  assert.match(installer, /let target_guard=replace_file/)
  assert.match(installer, /reader_sha256\(&mut guard\)/)
  assert.match(installer, /process::path\(launcher_pid\)/)
  assert.match(installer, /set_file_integrity\(source,"M"\)/)
  assert.doesNotMatch(installer, /start\(target/)
  assert.match(installer, /if job\.portable\{return Ok\(\(\)\);\}/)
  const recovery = installer.match(/pub fn recover_portable_exe[\s\S]*?\n\}/)?.[0] ?? ''
  assert.doesNotMatch(recovery, /start\(source/)
})

test('종료된 롤백과 준비 작업은 하루 뒤 캐시 정리 대상이다', () => {
  assert.match(installer, /Duration::from_secs\(24\*60\*60\)/)
  assert.match(installer, /abandoned_job\(phase,alive,stale,dir\.join\("portable-recovery\.json"\)\.exists\(\)\)/)
  assert.match(installer, /matches!\(phase,"preparing"\|"rolled-back"\)/)
})
