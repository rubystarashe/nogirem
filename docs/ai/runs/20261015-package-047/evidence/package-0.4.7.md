# 0.4.7 package evidence

- evidence_id: `EVID-PACKAGE-047-001`
- producer: `Cursor Agent`
- created_at: `2026-10-15T13:21:02Z`
- run_id: `20261015-package-047`
- checkpoint_id: `CP-PACKAGE-047`
- source_target: `9a64283a01121406f880206c3d9448d7329ea476`
- package_input_override: `사용자 NOTICE.md SHA-256 af1eb83b4ad8681314cf3123bf70f869f4d910434b401954a7c6825a8625f1f2`
- execution_target: `Windows 10.0.26200 x64, release/dioxus-0.4.7-2026-10-15T13-17-35-650Z`
- dependency_mode: `locked Rust release build, existing native helpers·NSIS, Microsoft WebView2 bootstrapper download, local Ed25519 release key`
- command: `npm run package:win`
- outcome: `PASS`, exit_code: `0`
- feature_impact: `FEAT-NOGIREM-TURBO-KEY — REMOVED helper·standalone terms package 배포 제외`
- feature_map: `updated — package evidence와 distribution exclusion 반영`
- architecture_impact: `signed Windows package·release assembly에서 제거 기능 binary/resource 차단`
- architecture_contract: `updated — revision 28 package 금지 경계`

## Artifacts

- package_directory: `release/dioxus-0.4.7-2026-10-15T13-17-35-650Z`
- installer: `nogirem-dioxus-setup-0.4.7.exe`, `9,555,690 bytes`, SHA-256 `081f0dc41f1f0d73f78f06cc42260e7bb2cc68d38293c1c762516deb24f443fd`
- portable: `nogirem-dioxus-portable-0.4.7.exe`, `7,773,971 bytes`, SHA-256 `6467b756bf013c404ebf09a1a6829af9214a5405704428494c0be2361459501f`
- installer_manifest: `update.json`; Ed25519 signature, version `0.4.7`, size와 SHA-256 일치 `PASS`
- portable_manifest: `portable-update.json`; Ed25519 signature, version `0.4.7`, size와 SHA-256 일치 `PASS`
- portable_cache_id: `55986698f937903aa1554775086be2551de931d0abdc96115ac2f8388468d796`

## Checks

- `PASS` — release build, packaged Rust service contract, Microsoft WebView2 Authenticode, NSIS `/WX`, installer 10MB 제한, manifest 생성·자체 검증
- `PASS` — native backend integration 3/3
- `PASS` — package·portable·release root에 `turbo-key-helper*.exe`, `TURBO_KEY_TERMS.md` 없음
- `PASS` — channel ping 두 HTML과 두 preload 존재
- `PASS` — 제거된 game overlay·debuff CSV·condition template 부재
- `PASS` — packaged `NOTICE.md`가 입력 SHA-256와 일치
- `PASS` — `runtimePackages=[]`
- `FAIL then PASS` — 최초 추가 verifier가 실제 파일명 `channel-ping-overlay-preload.js` 대신 잘못된 `channel-ping-preload.js`를 요구해 실패했다. 기대값을 실제 정본 이름으로 바로잡아 동일 산출물을 재검사했고 전 항목 통과했다. package 결함이나 산출물 변경은 없었다.

## Limitations

- 설치·제거·포터블 실행·자동 업데이트·게임·혼합 DPI 수동 시나리오는 `NOT_RUN`.
- package 산출물은 로컬에만 있으며 push·tag·GitHub release·배포를 수행하지 않았다.
- 공개키·SHA-256 등 공개 검증 정보만 기록했으며 private signing key는 기록하지 않았다.
