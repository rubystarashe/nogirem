# Follow-up — customized CPU settings indicator

- record_id: `TASK-CPU-CUSTOMIZED-005`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-CUSTOMIZED-005`
- owner/roles: `cursor-agent-92251f36 — Coordinator, DEV, DEV-as-QA, SELF_REVIEW`
- created_at: `2026-10-19T06:55:00Z`
- source_target: `base 60588e80b2002df7e160a4fa469e2301090e2dd9 + owned follow-up target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — customized count/safe combination visibility`
- feature_map: `updated — revision 6`
- architecture_impact: `advanced safe-mode control preserves custom count; main preset projection derives from effective settings`
- architecture_contract: `updated — revision 39`

## Request and risk

고급 기능에서 수동으로 선택한 core count와 안전 모드 사용 여부의 실제 조합이 `focus auto`, `balanced auto`, `safe auto` 중 어느 것과도 일치하지 않으면 메인 preset 위치에 `사용자화 설정됨`을 표시한다.

## Scope correction

- correction_at: `2026-10-19T07:01:00Z`
- requester correction: main preset control은 button group이 아니라 dropdown이어야 한다.
- final display: standard 상태에는 `게임 집중 / 균형 / 안전 모드`, custom 상태에는 `사용자화` option을 추가한 네 항목을 제공하고 `사용자화`를 현재 값으로 표시한다.
- supersedes: 아래 plan의 기존 standalone `사용자화 설정됨` label 표현.

`low-to-medium`: main 상태 projection과 기존 CPU setting writer의 좁은 control 하나를 바꾼다. topology mask, process writer, ledger, durable schema와 외부 integration은 바뀌지 않는다.

## Plan and acceptance

1. mode와 effective count를 focus/balanced default count와 비교해 exact preset을 계산한다.
2. exact preset이면 dropdown의 해당 standard option을 선택하고, 일치하지 않으면 네 번째 `사용자화` option을 추가해 선택한다.
3. 고급 `안전 모드 사용`은 custom count를 보존하며 safe↔balanced binding policy만 변경한다.
4. 수동으로 preset과 같은 count를 선택한 경우 실제 조합이 같으므로 customized로 표시하지 않는다.
5. desktop smoke에서 세 standard dropdown 전환을 확인한 뒤 advanced 5P+safe를 적용하고 `사용자화` option/value를 확인한다.

## Verification and review

- QA state: `COMPLETE_PASS`
- review_mode: `SELF_REVIEW`
- automated checks:
  - `cargo test --manifest-path desktop/backend/Cargo.toml --locked` — PASS.
  - `cargo test --manifest-path desktop/Cargo.toml --locked` — PASS, 8/8.
  - `nvm use 22.20.0; npm test` — PASS, 213/213.
  - `cargo check --manifest-path desktop/Cargo.toml --locked` — PASS after dropdown correction.
  - main desktop smoke with standard dropdown roundtrip, advanced 5P+safe preservation and four-option custom projection — PASS.
- evidence: `cpu-custom-dropdown-smoke.json`, `cpu-custom-dropdown.png`, `cpu-custom-dropdown-custom.png` in the active agent store.
- review: `REV-CPU-CUSTOMIZED-005-R2 — APPROVED, SELF_REVIEW`
- actual game performance: `NOT_RUN`
- out_of_scope: `NOTICE.md`, `data/`, push/deploy/release.
