# Follow-up — CPU options layout and preset feedback

- record_id: `TASK-CPU-UI-LAYOUT-006`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-UI-LAYOUT-006`
- owner/roles: `cursor-agent-92251f36 — Coordinator, DEV, DEV-as-QA, SELF_REVIEW`
- created_at: `2026-10-19T07:42:00Z`
- source_target: `base 60e83bbb97f86e96d9051f230c2afbaf54757b94 + owned follow-up target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — advanced CPU option hierarchy and preset interaction feedback`
- feature_map: `updated — revision 7`
- architecture_impact: `none — existing setting writers and service channels are unchanged; UI composition only`
- architecture_contract: `no_change — revision 39 remains accurate`

## Request and acceptance

- `고급 CPU 설정`을 game core count의 하위 heading이 아닌 별도 advanced option section으로 배치한다.
- main preset dropdown의 border와 background를 제거하고 font를 키우며 Vulkan control과의 상단 간격을 늘린다.
- preset service 요청이 pending인 동안 dropdown은 disabled 상태로 `설정 변경중`을 표시한다.

## Risk and plan

`low`: durable data, topology masks, service contracts, process roles와 외부 integration은 바뀌지 않는다. Dioxus component composition, CSS와 smoke assertions만 변경한다.

1. `CpuAdvancedSettings`를 별도 component와 `.developer-tool-stack`으로 분리한다.
2. preset dropdown은 transparent/borderless 11px style과 추가 vertical spacing을 사용한다.
3. pending state를 전용 option으로 투영한다.
4. smoke에서 두 CPU section이 다른 stack인지와 pending label을 검증한다.

## State

- DEV: `DONE`
- QA: `COMPLETE_PASS`
- review: `SELF_REVIEW APPROVED`
- checks:
  - `cargo check --manifest-path desktop/Cargo.toml --locked` — PASS.
  - `cargo test --manifest-path desktop/Cargo.toml --locked` — PASS, 8/8.
  - main desktop smoke — PASS; separate CPU stack indices, `설정 변경중`, standard/custom roundtrip.
  - edited-file IDE lints — PASS.
- visual_evidence: `cpu-ui-layout.png`; borderless transparent 11px dropdown and increased top spacing visually reviewed.
- limitations: actual game workload `NOT_RUN`; no independent reviewer.
- out_of_scope: `NOTICE.md`, `data/`, topology/worker behavior, push/deploy/release.
