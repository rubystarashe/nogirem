# 0.4.8 CPU 재정렬 가드 대치 배포

- record_id: `RUN-20261019-CPU-REORDER-048-HOTFIX`
- run_id: `20261019-cpu-reorder-048-hotfix`
- checkpoint_id: `CP-CPU-REORDER-GUARD-048`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-19T11:18:02Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `414c6b23d66c1a1925713a9206330f426a5a3ab5`
- request: `대체 P-core 그룹이 없을 때 CPU 재정렬 버튼을 비활성화하고 같은 0.4.8로 대치 배포`
- authorization: `implementation, local commit, origin/main push, signed 0.4.8 rebuild and existing GitHub v0.4.8 asset replacement`
- roles: `Coordinator=DEV=QA=Reviewer=Feature owner=Architecture owner=Documentation maintainer cursor-agent-92251f36`
- status: `COMPLETED`
- DEV: `DONE`
- QA: `COMPLETE_PASS for mandatory automated source, package and public checks`
- review_mode: `SELF_REVIEW`
- review_verdict: `APPROVED for implementation target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — 대체 P-core 그룹이 없는 CPU 재정렬 버튼 비활성화와 이유 표시`
- feature_map: `updated — canonical revision 37, feature revision 16`
- architecture_impact: `topology alternate-mask capability를 backend setting projection으로 전달하고 UI action을 차단`
- architecture_contract: `updated — revision 42`

## 범위·위험

- included: `canCpuReorder` capability projection, UI disabled state와 이유, fixture·topology·smoke 회귀, 0.4.8 변경 기록, signed package rebuild, 기존 public 0.4.8 Rust 자산 대치와 공개 재검증.
- excluded: CPU 재정렬 알고리즘 변경, tag 이동, Electron migration 자산 교체, 실제 게임 재정렬 실행.
- risk: `HIGH — UI safety hotfix는 작지만 이미 공개된 관리자 권한 0.4.8 실행 파일과 signed update manifests를 같은 버전으로 대치한다.`
- unrelated_preserved: 사용자 수정 `NOTICE.md`, 비추적 `data/`.

## 구현과 검증

- backend `core_setting`은 현재 mode·count topology에 겹치지 않는 동일 크기 alternate P-core mask가 있을 때만 `canCpuReorder=true`를 반환한다.
- UI는 safe·unsupported·alternate 부재 이유를 구분해 표시하고 capability가 false면 action을 보내지 못하게 한다.
- runtime `affinity::reorder`의 기존 fail-closed 검사는 defense-in-depth로 유지한다.
- fixture는 balanced 4/8P만 재정렬 가능하고 focus 6/8P·manual 5/8P·safe는 불가능하게 투영한다.
- `PASS` — backend 128 passed, 3 ignored, parity 2/2.
- `PASS` — desktop 8/8.
- `PASS` — Node 213/213.
- `PASS` — main UI smoke와 새 disabled reason assertion.
- `LIMITED` — full smoke는 main UI·startup·visual·creator·escape·boost까지 통과한 뒤 기존 terms 보조 실행이 종료되지 않아 timeout; 변경된 main UI 결과는 `passed=true`.
- `NOT_RUN` — 실제 마비노기에서 capability true 재정렬과 same-version 설치·업데이트.

## 계획과 중지 조건

1. `COMPLETED` — implementation·source QA·canonical docs.
2. `COMPLETED` — source checkpoint `d3e5a9c` commit·push.
3. `COMPLETED` — same-version signed package 생성과 검증.
4. `COMPLETED` — four executable aliases를 먼저, two signed manifests를 마지막에 대치하며 Electron 3개 자산 identity 보존.
5. `COMPLETED` — public 9/9 hash·Ed25519·aliases·feed와 release notes 재검증.
6. `COMPLETED` — final records commit·push.

- stop_condition: unsupported CPU reorder가 UI에서 사전 차단되고 source·package 검증이 통과하며 public 0.4.8 Rust 6개 자산이 대치본과 일치하고 Electron 3개 자산 identity가 보존되거나 정확한 blocker를 기록한다.

## 결과

- implementation_commit: `d3e5a9c5808d6dc897a0e0b7d2516373a2334c0d`
- package_target: `release/dioxus-0.4.8-2026-10-19T11-20-42-100Z`
- installer: `9,821,277 bytes`, SHA-256 `6c174a813e29d1f03a5c289297ade9a3a72e1df7fee6123b79620fcabcdf4c5b`
- portable: `7,836,103 bytes`, SHA-256 `87578abfc726f16a64d58f444a665637c6e8c5574deb3f7ad4d081fe201b61fb`
- publication_result: `PASS — public 9/9 candidate hash, Ed25519, aliases, feed ordering, Electron IDs and release notes`
- manual_QA: `NOT_RUN — 실제 game 재정렬, 설치·업데이트`
- gates: `feature PASS; architecture PASS; review/QA PASS for automated scope; target/authorization PASS`
- final_stop_condition: `satisfied for requested fix, same-version replacement and push`
