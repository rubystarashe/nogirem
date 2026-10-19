# Handoff — CPU topology and priority

- record_id: `HANDOFF-CPU-TOPOLOGY-PRIORITY-001`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-TOPOLOGY-PRIORITY-001`
- owner: `cursor-agent-92251f36`
- updated_at: `2026-10-19T05:35:00Z`
- status: `IN_PROGRESS`
- DEV: `IN_PROGRESS`
- QA: `COMPLETE_PASS`
- review: `APPROVED`, `SELF_REVIEW`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — CPU topology, affinity and optional background priority`
- feature_map: `updated — revision 31`
- architecture_impact: `topology authority, process adapter, durable ledger and IPC`
- architecture_contract: `updated — revision 35`

## Resume

- working_directory: `C:\Users\User\Documents\GitHub\nogirem`
- active_checkpoint: `CHECKPOINT_COMMIT`
- next_authorized_action: stage only owned files and create the authorized local checkpoint commit.
- target_manifest: `EVID-CPU-TOPOLOGY-SOURCE-001`, patch SHA-256 `f0f3b3d3c3eb1a869ea39f19cc59b569c2cff913865fd98604b128dfe0c30c18`.
- execution_target: `Windows 10.0.26200 x64; Rust test/debug target; Node v22.20.0`.
- preserved_out_of_scope: `NOTICE.md`, `data/`, any other baseline dirty content.
- blockers: none.
- completed: CPU Set+GLPIEx topology, disjoint L3-aware allocation, low-core/multi-group fail-safe, opt-in game binding and background priority, same-handle durable recovery, IPC/UI and canonical records.
- checks: backend 124 PASS·3 ignored plus parity 2/2; desktop 8/8; Node 213/213; desktop locked check PASS; diff check and IDE diagnostics PASS.
- limitations: actual game and packaged helper forced-exit recovery `NOT_RUN`; strict Clippy nonzero on disclosed pre-existing Rust 1.98 baseline warnings.
- findings: `ISSUE-CPU-001`~`003` resolved; no unresolved mandatory findings.
- commit/push/deploy: local commit pending; push/deploy not authorized.
