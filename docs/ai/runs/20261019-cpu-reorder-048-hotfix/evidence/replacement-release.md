# 0.4.8 CPU 재정렬 핫픽스 대치 배포 evidence

- evidence_id: `EVID-CPU-REORDER-048-REPLACEMENT-001`
- producer: `cursor-agent-92251f36`
- created_at: `2026-10-19T11:25:52Z`
- run_id: `20261019-cpu-reorder-048-hotfix`
- checkpoint_id: `CP-CPU-REORDER-GUARD-048`
- source_target: `d3e5a9c5808d6dc897a0e0b7d2516373a2334c0d`
- package_input_override: `사용자 NOTICE.md SHA-256 90b8803685fdb16e699c60b60d24d300738ff86bb8547a9a9eb481a2dc39cba3`
- package_target: `release/dioxus-0.4.8-2026-10-19T11-20-42-100Z`
- candidate_target: `release/ready-v0.4.8-2026-10-19T11-24-08-830Z`
- public_download: `release/public-v0.4.8-cpu-reorder-2026-10-19T11-25-00Z`
- release: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.8`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — 지원되지 않는 CPU 재정렬 사전 차단의 public 0.4.8 대치`
- feature_map: `updated — canonical revision 37, feature revision 16`
- architecture_impact: `기존 topology capability projection과 signed package/update 경계를 public target에 실행`
- architecture_contract: `no_change — revision 42를 exact package/public target에 재평가`

## 패키지

- installer: `nogirem-dioxus-setup-0.4.8.exe`, `9,821,277 bytes`, SHA-256 `6c174a813e29d1f03a5c289297ade9a3a72e1df7fee6123b79620fcabcdf4c5b`
- portable: `nogirem-dioxus-portable-0.4.8.exe`, `7,836,103 bytes`, SHA-256 `87578abfc726f16a64d58f444a665637c6e8c5574deb3f7ad4d081fe201b61fb`
- installer_manifest: `update.json`, SHA-256 `7b4a7a0eddb22d93dee92236328e916da9853a9db99e81e2f52c8512e350f180`
- portable_manifest: `portable-update.json`, SHA-256 `9f98ae7995fd20188ce83fabfac299114fde7e1bff9c4ed886778b6914754fb7`
- portable_cache_id: `e31401ed34c967d7cf35c1cb3d5defaf93a090235e57c0121e8c73a1e55777b3`

## 공개 대치

- `PASS` — source `d3e5a9c` origin/main push.
- `PASS` — canonical·alias 실행 파일 4개를 먼저 대치하고 signed manifest 2개를 마지막에 대치.
- `PASS` — 공개 9/9 candidate SHA-256 일치, installer·portable alias identity.
- `PASS` — 공개 Ed25519 manifests, version `0.4.8`, size와 SHA-256.
- `PASS` — Electron 3개 자산 ID `655808752`, `655808750`, `655808755`와 content identity 보존.
- `PASS` — annotated `v0.4.8` tag object와 target은 이동하지 않음.
- `PASS` — release note에 지원되지 않는 CPU 재정렬 실행 방지 반영.
- `PASS` — package Rust service contract, WebView2 signature, NSIS `/WX`, 10MB 제한, native integration 3/3.

## Source QA와 한계

- `PASS` — backend 128 passed, 3 ignored, parity 2/2; desktop 8/8; Node 213/213; main UI smoke.
- `LIMITED` — full smoke는 변경 main UI가 PASS한 뒤 후속 terms 보조 실행 timeout.
- `NOT_RUN` — 실제 마비노기에서 capability true 재정렬, 설치·제거·포터블 실행·same-version 자동 업데이트.
- review_mode: `SELF_REVIEW`; 독립 reviewer 미참여.
