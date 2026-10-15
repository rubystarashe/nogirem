# 0.4.7 graphics package evidence

- evidence_id: `EVID-REPACKAGE-047-GRAPHICS-001`
- producer: `cursor-agent-92251f36`
- created_at: `2026-10-15T14:33:12Z`
- run_id: `RUN-20261015-REPACKAGE-047-GRAPHICS`
- checkpoint_id: `CP-REPACKAGE-047-GRAPHICS`
- source_target: `nogirem main 62cf3ae48bb8a091bdb4d77a20c58f5cd7417f87`
- package_input_override: `NOTICE.md SHA-256 af1eb83b4ad8681314cf3123bf70f869f4d910434b401954a7c6825a8625f1f2`
- execution_target: `Windows 10.0.26200 x64; release/dioxus-0.4.7-2026-10-15T14-25-48-115Z`
- dependency_mode: `locked Rust release build, existing native helpers and NSIS, downloaded Microsoft WebView2 bootstrapper, local Ed25519 release key`
- feature_impact: `FEAT-NOGIREM-GRAPHICS-SETTINGS — restored mixed-GPU and AMD 3D implementation included in signed 0.4.7 package`
- feature_map: `updated — revision 3 with signed package evidence`
- architecture_impact: `Radeon helper identity, Rust-only runtime and signed Windows distribution boundary`
- architecture_contract: `no_change — revision 29 already defines the package and GPU helper boundaries`

## Artifacts

- package_directory: `release/dioxus-0.4.7-2026-10-15T14-25-48-115Z`
- installer: `nogirem-dioxus-setup-0.4.7.exe`, `9,568,848 bytes`, SHA-256 `3b1f78b97b675a6e1ac06c4eec191e97c719b40b58dca9170171af1ef474b8ef`
- portable: `nogirem-dioxus-portable-0.4.7.exe`, `7,786,663 bytes`, SHA-256 `8721f700865b1ff5db719ff326da06feae62239e1654d6b4f731c5ecbc8c8ca4`
- installer_manifest: `update.json`, Ed25519 signature and payload version·size·SHA-256 `PASS`
- portable_manifest: `portable-update.json`, Ed25519 signature and payload version·size·SHA-256 `PASS`
- portable_cache_id: `0855d4cdda3fcf1431a1d90149871b508437e62dd5cfdf635caa52995845b710`
- packaged_radeon_helper: app and portable SHA-256 `e1d3a09a9edf14be4f4914fbcfb4aab54d102b0f75edd83402ed6caae47643e8`, source helper와 일치

## Checks

- `PASS` — `npm run package:win`, exit `0`: release build, packaged native service contract, Microsoft WebView2 Authenticode, NSIS `/WX`, installer 10 MB budget and manifest generation/self-verification.
- `PASS 3/3` — `npm run test:backend-integration`.
- `PASS` — 독립 공개키 Ed25519 검증과 installer·portable version·size·SHA-256 재계산.
- `PASS` — app 53 files와 portable 55 files에서 required channel ping HTML·preload, Radeon helper, NOTICE 존재.
- `PASS` — `runtimePackages=[]`; Node/Electron backend payload, turbo helper, standalone `TURBO_KEY_TERMS.md`, 제거된 debuff overlay payload 부재.
- `PASS` — app·portable NOTICE hash가 지정된 사용자 입력과 일치.
- `FAIL then PASS` — 최초 verifier의 Windows path 정규화 문자열이 단일 `\`가 아니라 이중 `\\`를 찾도록 작성돼 required preload를 누락으로 보고했다. verifier만 수정해 같은 산출물을 재검사했으며 package 결함이나 산출물 변경은 없었다.

## Evidence limits

- 설치·제거·포터블 실행·자동 업데이트·게임·AMD 단독 GPU·혼합 DPI는 `NOT_RUN`.
- source behavior 검사는 implementation run의 exact committed target까지 포함되며 이번 checkpoint는 source 변경 없이 package assembly와 정적 payload를 검증했다.
- package는 로컬에만 있으며 push·tag·GitHub release·deployment는 `NOT_RUN`.
- private signing key는 기록하지 않았다.
