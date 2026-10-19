# Final report — 0.4.8 CPU 재정렬 핫픽스

- status: `COMPLETED`
- requested_scope: 대체 P-core 그룹이 없을 때 재정렬 UI를 사전 차단하고 같은 0.4.8로 대치 배포
- delivered_scope: topology capability projection, UI 비활성화·이유, source push, signed package rebuild, public Rust 6개 asset feed-safe 대치와 공개 재검증
- source_target: `d3e5a9c5808d6dc897a0e0b7d2516373a2334c0d`
- release: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.8`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — 지원되지 않는 CPU 재정렬 사전 차단의 public 0.4.8 대치`
- feature_map: `updated — canonical revision 37, feature revision 16`
- architecture_impact: `기존 topology capability projection과 signed package/update 경계를 public target에 실행`
- architecture_contract: `no_change — revision 42를 exact package/public target에 재평가`

## 구현·검증 결과

- implemented: alternate P-core mask가 없거나 safe·unsupported topology이면 CPU 재정렬 버튼을 비활성화하고 원인을 표시한다. runtime fail-closed 검사는 유지한다.
- automated_test_passing: backend 128 PASS·3 ignored와 parity 2/2, desktop 8/8, Node 213/213, main UI smoke.
- package_checks_passing: Rust service contract, WebView2 signature, NSIS `/WX`, 10MB 제한, native integration 3/3, Ed25519·size·hash.
- public_checks_passing: 9/9 candidate hash, aliases, signed manifests, release notes, unchanged Electron asset IDs·routing.
- installer: `9,821,277 bytes`, SHA-256 `6c174a813e29d1f03a5c289297ade9a3a72e1df7fee6123b79620fcabcdf4c5b`
- portable: `7,836,103 bytes`, SHA-256 `87578abfc726f16a64d58f444a665637c6e8c5574deb3f7ad4d081fe201b61fb`
- failed_or_limited_checks: initial backend `--locked` test는 누락된 standalone backend lock의 0.4.8 version 때문에 실패했고 lock을 동기화한 뒤 full backend PASS. Full smoke는 변경 main UI PASS 후 terms 보조 실행 timeout.
- manually_exercised: `NOT_RUN`
- not_run_checks: 실제 game capability true 재정렬, 설치·제거·포터블 실행·same-version 자동 업데이트

## 검토와 게이트

- review_mode: `SELF_REVIEW`
- review_verdict: `APPROVED`
- unresolved_mandatory_findings: none
- feature_gate: `PASS`
- architecture_gate: `PASS`
- review_qa_gate: `PASS for mandatory automated scope`
- target_authorization_gate: `PASS — 사용자 승인 범위 내 source push와 same-version asset replacement`
- remaining_risk: 동일 0.4.8을 이미 설치한 클라이언트는 update version 비교상 대치본을 자동 재수신하지 않을 수 있어 필요 시 수동 재설치해야 한다.
- commit_push_release_replacement: `COMPLETED`
