# Evidence — affinity single writer 0.4.9 package

- evidence_id: `EVID-AFFINITY-SINGLE-WRITER-049-PACKAGE-001`
- producer: `cursor-agent-92251f36`
- created_at: `2026-10-20T04:27:00Z`
- run_id: `20261020-affinity-single-writer-049`
- checkpoint_id: `CP-AFFINITY-SINGLE-WRITER-049`
- source_target: `640de3b4a3fd7ca646f4f8310a2ce7f71e0a653e`
- execution_target: `release/dioxus-0.4.9-2026-10-20T04-22-57-674Z`
- environment: `Windows 10.0.26200 x64, Node 22.20.0, Rust 1.98.1, NSIS 3.0.4.1`
- dependencies: `실제 Win32 process와 격리된 TEMP status/control/state; game·NIC mutation 없음`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — affinity helper 단일 writer와 시작 오류 진단 개선`
- feature_map: `updated — canonical revision 40, feature revision 17`
- architecture_impact: `helper lock exact process ownership과 startup status·stderr 경계`
- architecture_contract: `updated — revision 44`

## Source manifest

- `desktop/backend/src/storage.rs`: SHA-256 `79978f62518054dde9c862f363036a659126136ea52dbb7ea9eb86e676b62f04`
- `desktop/backend/src/boost.rs`: SHA-256 `dcaa4e0c50ebb797b0d50d6cb63d172a93c7efa3888175c164a26014bfce5c5b`
- `desktop/backend/src/memory.rs`: SHA-256 `09d216182322b6559c2a67a10aec7b71c9772eacbb11f0e998a17a1f4dbd65c9`
- `test/application-reliability.test.mjs`: SHA-256 `d0522802e7dcd139e3155c4e54addf5d8f5718a096fbeadcac259c00ff1f3707`
- `package.json`: SHA-256 `07123d9e33e2f820607b2904d0c0a617b7bf11c96f93d391c88303bfcfcf0714`
- `desktop/Cargo.toml`: SHA-256 `3a0b13714b415032773f4c17414f2fca1f550b4b86e0c5e825453c175de3a5e8`
- `desktop/backend/Cargo.toml`: SHA-256 `fc7e9a764b5bc2432ede269f66cf359dc8739c7acdf52974faa600f6b68658f4`
- `desktop/Cargo.lock`: SHA-256 `5b403d9ffc8366b05961ca970b7e4251e79d58796051f2135d9652981477d0bb`
- `desktop/backend/Cargo.lock`: SHA-256 `09438e3a633f8d2c46245fdc69b2ef3a50fa8e8d39f2212a18985665807e5e6e`
- `VERSION_HISTORY.md`: SHA-256 `61199b434a708f2ffad6d4f33bd20123dd78f73676a869c2fa1e6798cd18c1ea`
- unrelated_untracked: `data/` preserved and excluded.

## Package artifacts

- installer: `nogirem-dioxus-setup-0.4.9.exe`, `8,014,458 bytes`, SHA-256 `7c1c1dbcb78a544aa23c30ff637f2d824781da3362423d474fdee6d656804c28`
- portable: `nogirem-dioxus-portable-0.4.9.exe`, `7,789,775 bytes`, SHA-256 `dd3eb7f0ca119edc74ef31ca9b5236bac6a44153d16420db7a3eb99771fd0d67`
- installer manifest: `update.json`, SHA-256 `9e809e0609baaf3c94325d1fdf29a3f1c07b2c7ed306275a2fdc506f85d51bd2`
- portable manifest: `portable-update.json`, SHA-256 `1a909fa3b66da971c52f8f82ee818866ed6a7a35bd3a3f494316a588dfcb43a2`
- portable_cache_id: `5378f9d74cdbb3b7c96a48cf5420b4a62fdf1be101899658216a0493589c011c`

## Results

- `PASS` — backend 130 tests, 3 declared ignored; legacy/native parity 2/2.
- `PASS` — desktop 8/8.
- `PASS` — Node 213/213 after one stale static-contract assertion was updated and retested.
- `PASS` — native integration 3/3.
- `PASS` — signed package exit 0, non-solid NSIS, installer decimal 10MB gate.
- `PASS` — Ed25519 signatures, version `0.4.9`, payload size와 SHA-256.
- `PASS` — package `version.json=0.4.9`, recorder helper 1개, WebView2 bootstrapper 0개.
- `PASS` — installer·portable payload recorder helper와 committed helper의 SHA-256이 모두 `a3a16550cc915a1e9b0482c605b9777d51cfbc684f2feb61b182744a150020ba`로 일치.
- `PASS` — packaged first affinity helper running, second helper exit 1과 `Helper already running`, first helper graceful stop·final `running=false`.
- `FAIL, non-blocking pre-existing` — Rust 1.98 strict clippy는 변경 파일 밖 기존 lint 35건으로 실패했다. 변경 파일 finding은 없고 기존 debt를 수정·suppress하지 않았다.
- `NOT_RUN` — 실제 0.4.8 설치본 upgrade, 실제 game affinity 적용·복원, 설치·제거·포터블 UI.
- limitation: package build의 vendored Dioxus unused import·dead code warning 2건은 기존 warning이며 NSIS `/WX`와 package 결과를 차단하지 않았다.
