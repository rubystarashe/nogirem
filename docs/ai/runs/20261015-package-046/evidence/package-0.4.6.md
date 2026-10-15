# 0.4.6 package evidence

- evidence_id: `EVID-PACKAGE-046-001`
- producer: `Cursor Agent`
- created_at: `2026-10-15T05:59:18Z`
- run_id: `20261015-package-046`
- checkpoint_id: `CP-PACKAGE-046`
- source_target: `cdc477dd775849c7be102e4cc31a90473f74e171`
- package_input_override: `사용자 작성 NOTICE.md SHA-256 896a73660f459c4f13e01af0a5fb903ac47b639a981d29fbd6c0ea060d92920d`
- execution_target: `Windows 10.0.26200 x64, release/dioxus-0.4.6-2026-10-15T05-57-08-337Z`
- command: `npm run package:win`
- dependency_mode: `locked Rust release build, existing native helpers·NSIS, Microsoft WebView2 bootstrapper download, local Ed25519 release key`
- expected: 0.4.6 Rust release build, packaged native service contract, Microsoft WebView2 signature, NSIS `/WX`, installer 10 MB budget, installer·portable signed update manifest 자체 검증.
- actual: `exit code 0`; release build, packaged service contract, WebView2 signature 확인, 설치형·포터블 NSIS 생성, manifest 서명·자체 검증 완료.
- outcome: `PASS`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — tray lifecycle fix 포함; FEAT-NOGIREM-GAME-OVERLAY — 제거 상태 포함`
- feature_map: `no_change — canonical entries remain accurate`
- architecture_impact: `기존 signed package·native service contract·NSIS 생성 경계 실행`
- architecture_contract: `no_change — package·서명 경계 변경 없음`

## Artifacts

- package_directory: `release/dioxus-0.4.6-2026-10-15T05-57-08-337Z`
- installer: `nogirem-dioxus-setup-0.4.6.exe`, `9,572,765 bytes`, SHA-256 `f4b58b6477e118a7279481f8ce79394e6583487d8e3ee47da5da72bd0b3d076c`
- portable: `nogirem-dioxus-portable-0.4.6.exe`, `7,790,678 bytes`, SHA-256 `9a869dee1c6d0c9811c1f383fa278b43301a2364dd53d4ca54ec62efdff32ee1`
- installer_manifest: `update.json`; payload version·size·SHA-256가 installer와 일치하고 Ed25519 서명 자체 검증 PASS
- portable_manifest: `portable-update.json`; payload version·size·SHA-256가 portable과 일치하고 Ed25519 서명 자체 검증 PASS
- portable_cache_id: `2e7db9ae52f49fcca522f5d26717e254131dd23a921b84cc530067e4df03b8c6`

## Payload checks

- `version.json`: `0.4.6`
- `VERSION_HISTORY.md`: `0.4.6` 항목 포함
- packaged `NOTICE.md`: source input SHA-256와 일치
- channel files: `channel-ping-overlay.html`, `channel-ping-live.html`과 두 preload 존재
- removed game overlay files: settings·region·alert·configuration HTML, 전용 preload, `debuff.csv`, package condition templates 부재
- runtime_packages: `[]`
- installer_size_limit: `PASS — 9,572,765 < 10,000,000 bytes`
- sanitization: signing key 내용·경로를 기록하지 않음
- limitations: 실제 설치·제거·포터블 실행·업데이트·게임·혼합 DPI는 `NOT_RUN`
- push_status: `NOT_RUN`
- tag_status: `NOT_RUN`
- release_status: `NOT_RUN`
- deployment_status: `NOT_RUN`
