# Follow-up — focus priority preset and tighter arrow

- record_id: `TASK-CPU-FOCUS-PRIORITY-010`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-FOCUS-PRIORITY-010`
- owner/roles: `cursor-agent-92251f36 — Coordinator, DEV, DEV-as-QA, SELF_REVIEW`
- created_at: `2026-10-19T08:28:00Z`
- updated_at: `2026-10-19T08:38:00Z`
- source_target: `base 9b7a82ddd0d9bacab0a016d0a531eefe8c2e588e + owned working-tree target`
- status: `PARTIALLY_COMPLETED`
- DEV_state: `DONE`
- QA_state: `COMPLETE_PASS for automated source checks; UI smoke BLOCKED`
- review_state: `APPROVED`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — 게임 집중 preset이 일반 background priority 하향을 함께 적용하고 selector arrow·상단 간격을 조정`
- feature_map: `updated — revision 11`
- architecture_impact: `preset writer와 effective preset projection이 priority 값을 포함`
- architecture_contract: `updated — revision 40`

## 요청·범위·위험

- original_request: `아래방향아이콘 왼쪽으로 좀더 이동시켜. 여전히 사이 간격 크다`; `게임 집중 모드 프리셋인 경우 우선순위옵션까지 키는게 좋을거같은데 그럼`; `부스트모드 선택 상단 마진 살짝 더 줘`
- objective: selector field를 110px에서 100px로 줄여 chevron을 약 10px 왼쪽으로 옮기고 top을 105px에서 109px로 내린다. focus preset만 `lowerBackgroundPriority=true`를 적용한다.
- preset_contract: focus는 자동 focus core count, game hard affinity, background priority 하향을 적용한다. balanced와 safe preset은 priority 하향을 끈다. 고급 기능에서 이 값을 다르게 바꾸면 main selector는 `사용자화`를 표시한다.
- migration: 새 설정과 priority 필드가 없는 focus 설정은 true를 기본값으로 사용한다. 명시적으로 저장된 false는 보존해 사용자화를 유지한다.
- exclusions: priority 적용 대상 분류, affinity mask, process ledger·복원, 배포·push·release.
- risk: `LOW — 기존 boolean writer와 worker 제어를 preset 조합에 포함하고 CSS 폭만 변경한다. durable schema, process 대상, topology와 보안 경계는 바뀌지 않는다.`

## 구현·검증

- changed_files: `web/styles.css`, `desktop/src/ui.rs`, `desktop/src/smoke.rs`, `desktop/backend/src/settings.rs`, `boost.rs`, `service_fixture.rs`, canonical/run/handoff records.
- backend locked tests: `PASS — 127 passed, 3 ignored, parity 2/2`.
- desktop locked tests: `PASS — 8/8`.
- desktop locked check: `PASS`.
- targeted settings test: `PASS — 1/1`.
- edited-file lints and `git diff --check`: `PASS`.
- UI smoke: `BLOCKED — 기존 관리자 권한 debug 앱 4개가 target executable을 잠가 두 번 access denied; 별도 target build는 임시 디스크 공간 부족으로 중단했고 생성된 1.8GiB를 cargo clean으로 제거했다.`
- actual game workload: `NOT_RUN`.

## SELF_REVIEW

- review_id: `REV-CPU-FOCUS-PRIORITY-010-R1`
- target: 위 source target의 실제 diff와 관련 settings/preset/fixture/UI 경로.
- findings: `mandatory finding 없음`.
- assessment: preset 선택은 저장 전에 priority 값을 명시하고 같은 `set-cpu-options` runtime transaction으로 전달한다. 기존 explicit false를 정규화 단계에서 덮어쓰지 않는다. UI preset 판정은 mode/count/priority 세 값이 모두 일치할 때만 standard preset으로 표시한다. CSS 변경은 selector의 위치와 폭에만 한정된다.
- limitations: 독립 reviewer 없음. 잠긴 관리자 앱 때문에 변경 target의 WebView2 smoke와 screenshot은 실행하지 못했다.
- verdict: `APPROVED`.

## Final disposition

- disposition: `PARTIALLY_COMPLETED — 구현과 자동 source 검증은 완료, 변경 target UI smoke는 BLOCKED`
- feature_gate: `PASS for source contract; actual game performance NOT_RUN`
- architecture_gate: `PASS — 기존 single writer와 worker transaction 유지`
- review_gate: `PASS — SELF_REVIEW, mandatory finding 없음`
- QA_gate: `PASS for automated tests/check/lints; UI smoke BLOCKED`
- target_gate: `PASS after the local checkpoint containing this record; resulting SHA is reported externally`
- push/deploy/release: `NOT_RUN — 승인되지 않음`
