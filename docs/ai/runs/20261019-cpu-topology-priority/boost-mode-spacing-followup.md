# Follow-up — boost mode chevron and spacing

- record_id: `TASK-CPU-DROPDOWN-008`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-DROPDOWN-008`
- owner/roles: `cursor-agent-92251f36 — Coordinator, DEV, DEV-as-QA, SELF_REVIEW`
- created_at: `2026-10-19T08:05:00Z`
- source_target: `base 593bd05a5829b07a951583429308fd35930a450c + owned follow-up target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — boost mode chevron/spacing and safe-mode terminology`
- feature_map: `updated — revision 9`
- architecture_impact: `none — visual and user-facing copy only`
- architecture_contract: `no_change — revision 39 remains accurate`

## Request and acceptance

- glyph dropdown icon을 CSS border chevron으로 교체한다.
- Vulkan button과 전체 control 사이 vertical margin은 늘리고 ruby label과 select 사이 간격은 줄인다.
- 안전 모드 설명의 사용자 노출 `affinity`를 `게임 CPU 선호도`로 바꾼다.
- smoke에서 Vulkan gap, ruby gap, chevron border와 user-facing terminology를 검증한다.

## Risk and state

- risk: `low`; service, setting, topology, process behavior와 durable data는 변경하지 않는다.
- DEV: `DONE`
- QA: `COMPLETE_PASS`
- review: `SELF_REVIEW APPROVED`
- checks:
  - desktop locked check — PASS.
  - main desktop smoke — PASS; Vulkan gap 10px, ruby gap ≤0px, 1px CSS chevron borders and `게임 CPU 선호도` copy.
  - edited-file IDE lints — PASS.
- visual_evidence: `boost-mode-spacing.png`; spacing and chevron visually reviewed.
- limitations: actual game workload `NOT_RUN`; no independent reviewer.
- out_of_scope: `NOTICE.md`, `data/`, CPU behavior, push/deploy/release.
