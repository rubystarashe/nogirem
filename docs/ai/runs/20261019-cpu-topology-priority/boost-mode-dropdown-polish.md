# Follow-up — boost mode dropdown polish

- record_id: `TASK-CPU-DROPDOWN-007`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-DROPDOWN-007`
- owner/roles: `cursor-agent-92251f36 — Coordinator, DEV, DEV-as-QA, SELF_REVIEW`
- created_at: `2026-10-19T07:53:00Z`
- source_target: `base f8c4411 + owned follow-up target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — main boost mode presentation and top-level advanced CPU options`
- feature_map: `updated — revision 8`
- architecture_impact: `none — presentation-only refinement`
- architecture_contract: `no_change — revision 39 remains accurate`

## Request and acceptance

- native select의 focus/highlight box를 제거하고 transparent 상태를 유지한다.
- mode text를 `게임 집중 모드 / 균형 모드 / 안전 모드`로 통일한다.
- control left inset을 줄이고 상단에 작은 `부스트 모드` ruby-style label을 둔다.
- boost active/paused 전환에서 select, label, arrow color가 자연스럽게 transition한다.

## Scope correction

- correction_at: `2026-10-19T07:55:00Z`
- `안전 모드 사용`과 `일반 백그라운드 프로세스 우선순위 낮춤`을 묶음 하위 checkbox가 아닌 각각의 top-level option으로 표시한다.
- 두 option은 다른 advanced 기능과 동일한 `사용하기 / 사용 중` button interaction을 사용한다.
- direct order는 `게임 CPU 코어 할당 개수 → 안전 모드 사용 → 일반 백그라운드 프로세스 우선순위 낮춤 → CPU 재정렬`이다.

## Risk and plan

`low`: UI text, CSS presentation과 existing actions의 composition만 바뀐다. setting writer, service channel, CPU mask, process role, durable data와 외부 integration은 변경하지 않는다.

1. native appearance·outline·shadow를 제거하고 별도 arrow를 사용한다.
2. label과 option text를 갱신한다.
3. computed style과 labels를 desktop smoke에서 검증한다.
4. top-level action type, labels와 direct ordering을 smoke에서 검증한다.

## State

- DEV: `DONE`
- QA: `COMPLETE_PASS`
- review: `SELF_REVIEW APPROVED`
- checks:
  - desktop locked check — PASS.
  - desktop tests — PASS, 8/8.
  - main desktop smoke — PASS; labels/computed style/pending/custom roundtrip and top-level CPU order/actions.
  - edited-file IDE lints — PASS.
- visual_evidence: `boost-mode-polish.png`; no box/background, ruby label, custom arrow and left inset visually reviewed.
- limitations: actual game workload `NOT_RUN`; no independent reviewer.
- out_of_scope: `NOTICE.md`, `data/`, CPU behavior, push/deploy/release.
