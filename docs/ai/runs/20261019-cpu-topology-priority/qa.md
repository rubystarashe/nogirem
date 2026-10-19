# QA — CPU topology and priority

- record_id: `QA-CPU-TOPOLOGY-PRIORITY-001`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-TOPOLOGY-PRIORITY-001`
- owner: `cursor-agent-92251f36`
- qa_role: `DEV-as-QA`
- qa_state: `COMPLETE_PASS`
- source_target: `EVID-CPU-TOPOLOGY-SOURCE-001; patch SHA-256 f0f3b3d3c3eb1a869ea39f19cc59b569c2cff913865fd98604b128dfe0c30c18`
- execution_target: `Windows 10.0.26200 x64; Rust test/debug target; Node v22.20.0`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — topology allocation, affinity and priority recovery`
- feature_map: `no_change — QA intake only; canonical update follows verified implementation`
- architecture_impact: `topology parser/allocation and process mutation/restore boundaries`
- architecture_contract: `no_change — QA intake only; canonical update follows verified implementation`

## 필수 시나리오

- `SCN-CPU-TOPO-001`: hybrid SMT fixture에서 요청 P-core 수와 disjoint background mask.
- `SCN-CPU-TOPO-002`: AMD single/multiple L3와 SMT on/off locality.
- `SCN-CPU-TOPO-003`: 2C/4T 및 4 physical core fail-safe.
- `SCN-CPU-TOPO-004`: malformed API buffer와 multi-group fail-safe.
- `SCN-CPU-AFF-001`: game binding 기본 false와 opt-in 적용.
- `SCN-CPU-PRI-001`: background priority 기본 false 및 role exclusion.
- `SCN-CPU-REC-001`: normal restore, PID identity mismatch, external mutation preservation and restart recovery.
- `SCN-CPU-IPC-001`: persisted settings, live worker control, service contract and UI status.
- `SCN-CPU-LIVE-001`: Windows child process priority roundtrip.
- `SCN-CPU-GAME-001`: 실제 게임 frame 성능 비교. `NOT_RUN` 예정이며 자동 검증 범위 밖.

## 실행 기록

- `SCN-CPU-TOPO-001`~`004`: `PASS` — `cargo test --locked --manifest-path desktop/backend/Cargo.toml`; hybrid, L3, SMT, 4-core, malformed, multi-group와 live Windows topology 포함.
- `SCN-CPU-AFF-001`: `PASS` — default false settings test, typed worker options, service/UI contract, SELF_REVIEW.
- `SCN-CPU-PRI-001`: `PASS` — role target unit test와 child process `BELOW_NORMAL_PRIORITY_CLASS` roundtrip.
- `SCN-CPU-REC-001`: `PASS` — same-handle implementation/source review, external-value restore guard unit test, durable ledger startup path review.
- `SCN-CPU-IPC-001`: `PASS` — Node `npm test` 213/213, desktop 8/8 and locked desktop check.
- `SCN-CPU-LIVE-001`: `PASS` — `process::tests::child_priority_roundtrip_uses_one_verified_handle`.
- `SCN-CPU-GAME-001`: `NOT_RUN` — 실제 마비노기와 packaged helper 강제 종료 환경이 없어 자동 검사와 분리.

## 명령과 결과

- `cargo test --locked --manifest-path desktop/backend/Cargo.toml`: `PASS`, library 124 passed·3 ignored, integration parity 2/2, doc tests 0.
- `cargo test --locked --manifest-path desktop/Cargo.toml`: `PASS`, 8/8.
- `cargo check --locked --manifest-path desktop/Cargo.toml`: `PASS`.
- `nvm current; npm test`: `PASS`, Node v22.20.0, 213/213.
- `git diff --check`: `PASS`; line-ending conversion notices only.
- IDE diagnostics on changed Rust/UI files: `PASS`, 0 errors.
- `cargo clippy --locked --manifest-path desktop/Cargo.toml --all-targets -- -D warnings`: `FAIL` due repository baseline warnings under Rust 1.98 in unrelated pre-existing code. The one introduced `too_many_arguments` finding was fixed with typed `Options`; strict command remains nonzero because baseline warnings were not broadened or suppressed.

## Coverage assessment

Mandatory source and automated scenarios for this checkpoint passed. Actual game frame/latency behavior remains explicitly `NOT_RUN` and no frame-improvement claim is made. Strict Clippy is a disclosed repository-baseline gap rather than a new violation in the changed implementation.
