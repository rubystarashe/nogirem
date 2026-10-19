# Follow-up — boost mode label and arrow inset

- record_id: `TASK-CPU-DROPDOWN-009`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-DROPDOWN-009`
- owner/roles: `cursor-agent-92251f36 — Coordinator, DEV, DEV-as-QA, SELF_REVIEW`
- created_at: `2026-10-19T08:16:00Z`
- source_target: `base 4df74e239ffaad517911a46f62682958337fdeec + owned follow-up target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — boost mode label and compact arrow placement`
- feature_map: `updated — revision 10`
- architecture_impact: `none — visual text and spacing only`
- architecture_contract: `no_change — revision 39 remains accurate`

## Request and acceptance

- ruby label을 `부스트 모드 선택`으로 바꾼다.
- dropdown width와 right padding을 줄여 chevron을 선택값 쪽으로 당긴다.
- smoke에서 110px field, 15px right padding, 6px arrow inset과 label을 확인한다.

## State

- risk: `low`; product behavior, service, durable data와 integration은 변경하지 않는다.
- DEV: `DONE`
- QA: `COMPLETE_PASS`
- review: `SELF_REVIEW APPROVED`
- checks:
  - desktop locked check — PASS.
  - main desktop smoke — PASS after one transient WebView2 tab-transition failure; clean-process rerun passed.
  - edited-file IDE lints — PASS.
- visual_evidence: `boost-mode-label.png`; compact chevron placement and label visually reviewed.
- limitations: actual game workload `NOT_RUN`; no independent reviewer.
- out_of_scope: `NOTICE.md`, `data/`, CPU behavior, push/deploy/release.
