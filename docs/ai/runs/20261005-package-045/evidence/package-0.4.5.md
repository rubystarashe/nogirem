# EVIDENCE-PACKAGE-045

- evidence_id: `EVIDENCE-PACKAGE-045`
- producer: `Cursor Agent`
- created_at: `2026-10-05T08:24:03Z`
- run_id: `20261005-package-045`
- checkpoint_id: `CP-PACKAGE-045`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — 0.4.5 실시간 게임 서버 모니터링과 채널별 서버 점수를 설치형·포터블 payload에 포함`
- feature_map: `no_change — canonical behavior는 source와 일치하며 packaging evidence만 추가`
- architecture_impact: `기존 signed package·native service contract·NSIS 생성 경계 실행`
- architecture_contract: `no_change — package writer와 서명 경계를 변경하지 않음`
- source_target: `4c11b0e42055073597d080cf59219d22cd06cfbb`
- execution_target: `Windows 10.0.26200 x64, release/dioxus-0.4.5-2026-10-05T08-21-42-249Z`
- command: `npm run package:win`
- dependency_mode: `local locked Rust build, 기존 native helper·NSIS, Microsoft WebView2 bootstrapper download, 로컬 Ed25519 release key`
- expected: Rust release build, packaged native service contract, WebView2 signature, NSIS `/WX`, installer 10 MB budget, installer·portable signed update manifest 자체 검증.
- actual: `exit code 0`; release build, packaged service contract, Microsoft signature 확인, 설치형·포터블 NSIS 생성, manifest 서명·자체 검증 완료.
- outcome: `PASS`

## Artifacts

- directory: `release/dioxus-0.4.5-2026-10-05T08-21-42-249Z`
- installer: `nogirem-dioxus-setup-0.4.5.exe`
  - size: `9,559,488 bytes`
  - SHA-256: `4a3aba2e38262ee086c7d8b012a2a0bed2746664903e44a4ce08a54ef63a5089`
- portable: `nogirem-dioxus-portable-0.4.5.exe`
  - size: `7,783,890 bytes`
  - SHA-256: `9033ba9f42f49c4d66de7f859213940f2b9c150a4f518723aa3d2cc620d4c274`
- installer manifest: `update.json`
  - SHA-256: `020d4cead08fb5b60946dac27396e5319bf74e3dba8f5722446343d9d612cb49`
- portable manifest: `portable-update.json`
  - SHA-256: `79d3d798bf225be46f5cb056f097c7698d3cd3158a3bd462462bac8ae346eeed`
- build metadata: `build.json`
  - SHA-256: `7ada59949ed4d569690ade2a834d657c43427149f102868c05ecfef95d80268d`
- portable cache identity: `aba8d02a2a7945b81abab9df124c091e5c82efcd86ca7b6f94a6c3e6e6142944`

## Verification

- `npm test`: `PASS — 204/204`
- `cargo test --locked --manifest-path desktop/Cargo.toml`: `PASS — 8/8`
- `cargo test --locked -p nogirem-backend`: `PASS — unit 93/93, declared ignore 2, integration 2/2`
- `node --test test/overlay-interface.test.mjs`: `PASS — 3/3`
- manifest payload version·size·SHA-256 versus generated EXE: `PASS — 2/2`
- required payload: `channel-ping-overlay.html`, `channel-ping-live.html`, 두 read-only preload, `channel.csv` 포함 확인.
- packaged central title: `채널별 서버 점수` 확인.
- forbidden Node/Electron backend runtime: package script recursive verification `PASS`.
- initial `cargo test --locked --manifest-path desktop/backend/Cargo.toml`: `NOT_RUN — nested manifest의 별도 lock 갱신 거부로 테스트 시작 전 종료; workspace package 명령으로 대체해 전체 backend 통과`.

## Limitations

- 설치형 설치·제거·업데이트, 포터블 실행, 실제 게임 전경·물리 입력·혼합 DPI는 `NOT_RUN`.
- GitHub 업로드·공개 다운로드, push·tag·release·배포는 `NOT_RUN`.
- signing key 내용과 경로는 기록하지 않음.
