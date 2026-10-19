# 0.4.8 package evidence

- evidence_id: `EVID-PACKAGE-048-001`
- producer: `cursor-agent-92251f36`
- created_at: `2026-10-19T09:14:20Z`
- run_id: `20261019-package-048`
- checkpoint_id: `CP-PACKAGE-048`
- source_target: `af36bc1e1e905864914c8fea4bfd2e7a17cd25fa`
- package_input_override: `사용자 NOTICE.md SHA-256 90b8803685fdb16e699c60b60d24d300738ff86bb8547a9a9eb481a2dc39cba3, 1413 bytes`
- execution_target: `Windows 10.0.26200 x64, release/dioxus-0.4.8-2026-10-19T09-10-32-162Z`
- dependency_mode: `locked Rust release build, existing native helpers·NSIS, Microsoft WebView2 bootstrapper download, local Ed25519 release key`
- command: `npm run package:win`
- outcome: `PASS`, exit_code: `0`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — 균형 기본 CPU 모드가 포함된 0.4.8 배포 후보`
- feature_map: `updated — revision 34, feature revision 13에 package evidence 연결`
- architecture_impact: `none — 기존 signed Windows package 경로와 payload 경계를 변경하지 않음`
- architecture_contract: `no_change — revision 41을 재검토했고 현재 package 구조와 일치`

## 산출물

- package_directory: `release/dioxus-0.4.8-2026-10-19T09-10-32-162Z`
- installer: `nogirem-dioxus-setup-0.4.8.exe`, `9,823,145 bytes`, SHA-256 `3c697b541da666ed4c1127bbc2cfc73b0eb58a0d89dc8f84369fbe9fc6f0e0fe`
- portable: `nogirem-dioxus-portable-0.4.8.exe`, `7,836,262 bytes`, SHA-256 `cf3d565bcda0888290e1e21934d14773313418623d2b521f72e0bfb67887415e`
- installer_manifest: `update.json`; Ed25519 signature, version `0.4.8`, size와 SHA-256 일치 `PASS`
- portable_manifest: `portable-update.json`; Ed25519 signature, version `0.4.8`, size와 SHA-256 일치 `PASS`
- portable_cache_id: `111aa51379eea80e5a2e55486ef3625e67f240159a819d532a279c30c99c18e3`

## 검사

- `PASS` — release build, packaged Rust service contract, Microsoft WebView2 Authenticode, NSIS `/WX`, installer 10MB 제한, manifest 생성·자체 검증
- `PASS` — 독립 공개키 Ed25519·version·size·SHA-256 재검증
- `PASS` — native backend integration 3/3
- `PASS` — Node 213/213, desktop compile
- `PASS` — app·portable `version.json`과 0.4.8 변경 기록
- `PASS` — package의 app·portable `NOTICE.md`가 사용자 입력 SHA-256와 일치
- `PASS` — channel ping 두 HTML과 두 preload 존재
- `PASS` — Node/Electron backend, turbo helper·독립 약관, 제거된 game overlay·debuff·condition payload 부재
- `PASS` — `runtimePackages=[]`
- verifier_note: 최초 inline Node 검증 명령은 PowerShell이 JavaScript template literal의 `$`를 확장해 syntax error로 실패했다. template literal을 사용하지 않는 동일 검증을 즉시 재실행해 `PASS`했으며 산출물 결함이나 변경은 없었다.

## 한계

- 설치·제거·포터블 실행·자동 업데이트·게임 수동 시나리오는 `NOT_RUN`.
- package 산출물은 로컬에만 있으며 push·tag·GitHub Release·배포를 수행하지 않았다.
- 공개키·SHA-256 등 공개 검증 정보만 기록했으며 private signing key는 기록하지 않았다.
