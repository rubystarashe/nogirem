# Coordination — CPU topology and priority

- record_id: `COORD-CPU-TOPOLOGY-PRIORITY-001`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-TOPOLOGY-PRIORITY-001`
- owner: `cursor-agent-92251f36`
- updated_at: `2026-10-19T05:35:00Z`
- exact_target: `EVID-CPU-TOPOLOGY-SOURCE-001; patch SHA-256 f0f3b3d3c3eb1a869ea39f19cc59b569c2cff913865fd98604b128dfe0c30c18`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — CPU allocation and background priority behavior`
- feature_map: `updated — revision 31`
- architecture_impact: `topology/process/ledger/service boundaries`
- architecture_contract: `updated — revision 35`

## 역할과 소유권

- Coordinator, DEV, QA, Reviewer, Feature owner, Architecture owner, Documentation maintainer: `cursor-agent-92251f36`
- review_mode: `SELF_REVIEW`
- 독립 reviewer: `none`
- 코드·run record 활성 writer: `cursor-agent-92251f36`
- 사용자 소유로 보존: `NOTICE.md`, `data/`, 그 밖의 baseline dirty files.

## 현재 상태

- active_checkpoint: `COMPLETE`
- DEV: `IN_PROGRESS`
- QA: `COMPLETE_PASS`
- review: `APPROVED`, `SELF_REVIEW`
- stop_condition: 구현, mandatory checks, SELF_REVIEW, canonical docs, exact-target manifest와 local checkpoint commit 완료.

## 결정과 경계

- multi-group CPU에서 단일 `u64` process affinity mask를 적용하지 않는다.
- background만 기본 이동하고 game binding은 명시적 opt-in이다.
- priority 변경은 일반 background role에만 opt-in으로 적용한다.
- ledger restore는 동일 process identity이며 현재 값이 앱 적용값과 일치할 때만 수행한다.
- 외부 게시·push·배포·release는 권한 범위 밖이다.

## 동시 작업과 미해결 항목

- ownership overlap: `NOTICE.md`와 `data/` 제외로 해소.
- exact source target, QA evidence and self-review: 확정.
- local checkpoint commit: pending.
- cross-repository dependency: 없음.

## Follow-up CP-CPU-MODES-002

- updated_at: `2026-10-19T06:27:00Z`
- active_checkpoint: `VERIFY_REVIEW`
- DEV: `READY_FOR_REVIEW`
- QA: `COMPLETE_PASS`
- review: `APPROVED`, `SELF_REVIEW`
- exact_target: `base 17dee9063db8c9926d70a1f9eab5a4955b7c47d8 + follow-up owned-file target; final manifest pending`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — 게임 집중·균형·안전 mode`
- feature_map: `updated — revision 3`
- architecture_impact: `mode allocation, normalized setting and live control`
- architecture_contract: `updated — revision 36`
- scope_decision: 원 계획의 non-overlap·저코어 disable·game binding default false는 사용자 후속 성능 정책으로 대체됐다. 원 `plan.md`는 불변 기록으로 유지하고 `request-mode-followup.md`와 `mode-plan.md`가 후속 권한을 기록한다.
- ownership: `cursor-agent-92251f36`가 follow-up code와 run/canonical docs의 유일 active writer다. `NOTICE.md`, `data/`는 계속 사용자 소유로 제외한다.
- next_action: exact manifest와 handoff를 갱신하고 명시 파일만 local checkpoint commit한다.

## Final correction CP-CPU-MAIN-PRESETS-004

- updated_at: `2026-10-19T06:45:00Z`
- supersedes: `CP-CPU-MODES-002`, `CP-CPU-MODE-SPLIT-003` for final product behavior
- active_checkpoint: `COMPLETE`
- DEV: `READY_FOR_REVIEW`
- QA: `COMPLETE_PASS`
- review: `APPROVED`, `SELF_REVIEW`
- exact_target: `EVID-CPU-MAIN-PRESETS-SOURCE-004; patch SHA-256 27fff031e3e02736c5feebbef90d495aa640e1c72ba70aad306ce6b4842fcb6c`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — main three presets and advanced custom count`
- feature_map: `updated — revision 5`
- architecture_impact: `preset orchestration and main UI placement`
- architecture_contract: `updated — revision 38`
- scope_decision: final authority는 `request-main-presets.md`와 `main-presets-plan.md`다. 이전 follow-up 문서는 당시 사용자 지시의 immutable history로 유지한다.
- ownership: source/run/canonical active writer는 `cursor-agent-92251f36`; `NOTICE.md`, `data/`는 제외한다.

## Follow-up CP-CPU-CUSTOMIZED-005

- updated_at: `2026-10-19T07:05:00Z`
- active_checkpoint: `COMPLETE`
- DEV: `DONE`
- QA: `COMPLETE_PASS`
- review: `REV-CPU-CUSTOMIZED-005-R2 APPROVED`, `SELF_REVIEW`
- exact_target: `base 60588e80b2002df7e160a4fa469e2301090e2dd9 + owned customized-dropdown target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — standard/custom CPU preset dropdown`
- feature_map: `updated — revision 6`
- architecture_impact: `preserving advanced safe toggle and effective preset projection`
- architecture_contract: `updated — revision 39`
- open mandatory findings: `none`
- ownership: same single-agent writer; `NOTICE.md` and `data/` remain out of scope.

## Follow-up CP-CPU-UI-LAYOUT-006

- updated_at: `2026-10-19T07:47:00Z`
- active_checkpoint: `CHECKPOINT_COMMIT`
- DEV: `DONE`
- QA: `COMPLETE_PASS`
- review: `REV-CPU-UI-LAYOUT-006-R1 APPROVED`, `SELF_REVIEW`
- exact_target: `base 60e83bbb97f86e96d9051f230c2afbaf54757b94 + owned UI-layout target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — advanced CPU hierarchy and preset feedback`
- feature_map: `updated — revision 7`
- architecture_impact: `none — UI composition only`
- architecture_contract: `no_change — revision 39 remains accurate`
- open mandatory findings: `none`
- ownership: single-agent writer; `NOTICE.md` and `data/` remain out of scope.

## Follow-up CP-CPU-DROPDOWN-009

- updated_at: `2026-10-19T08:19:00Z`
- active_checkpoint: `COMPLETE`
- DEV: `DONE`
- QA: `COMPLETE_PASS`
- review: `REV-CPU-DROPDOWN-009-R1 APPROVED`, `SELF_REVIEW`
- exact_target: `base 4df74e239ffaad517911a46f62682958337fdeec + owned label/inset target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — boost mode label and compact arrow placement`
- feature_map: `updated — revision 10`
- architecture_impact: `none — visual text and spacing only`
- architecture_contract: `no_change — revision 39 remains accurate`
- open mandatory findings: `none`
- ownership: single-agent writer; `NOTICE.md` and `data/` remain out of scope.

## Follow-up CP-CPU-DROPDOWN-008

- updated_at: `2026-10-19T08:10:00Z`
- active_checkpoint: `COMPLETE`
- DEV: `DONE`
- QA: `COMPLETE_PASS`
- review: `REV-CPU-DROPDOWN-008-R1 APPROVED`, `SELF_REVIEW`
- exact_target: `base 593bd05a5829b07a951583429308fd35930a450c + owned spacing/copy target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — boost mode chevron/spacing and safe-mode terminology`
- feature_map: `updated — revision 9`
- architecture_impact: `none — visual and copy only`
- architecture_contract: `no_change — revision 39 remains accurate`
- open mandatory findings: `none`
- ownership: single-agent writer; `NOTICE.md` and `data/` remain out of scope.

## Follow-up CP-CPU-DROPDOWN-007

- updated_at: `2026-10-19T07:59:00Z`
- active_checkpoint: `CHECKPOINT_COMMIT`
- DEV: `DONE`
- QA: `COMPLETE_PASS`
- review: `REV-CPU-DROPDOWN-007-R1 APPROVED`, `SELF_REVIEW`
- exact_target: `base f8c4411 + owned dropdown/action target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — boost mode presentation and top-level advanced CPU options`
- feature_map: `updated — revision 8`
- architecture_impact: `none — UI composition only`
- architecture_contract: `no_change — revision 39 remains accurate`
- open mandatory findings: `none`
- ownership: single-agent writer; `NOTICE.md` and `data/` remain out of scope.
