# Handoff — CPU topology and priority

- record_id: `HANDOFF-CPU-TOPOLOGY-PRIORITY-001`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-MAIN-PRESETS-004`
- owner: `cursor-agent-92251f36`
- updated_at: `2026-10-19T06:50:00Z`
- status: `IN_PROGRESS`
- DEV: `IN_PROGRESS`
- QA: `COMPLETE_PASS`
- review: `APPROVED`, `SELF_REVIEW`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — main three presets and advanced manual P-core count`
- feature_map: `updated — feature revision 5`
- architecture_impact: `preset defaults, topology classification, settings/worker IPC and main UI placement`
- architecture_contract: `updated — revision 38`

## Resume

- working_directory: `C:\Users\User\Documents\GitHub\nogirem`
- active_checkpoint: `CHECKPOINT_COMMIT`
- next_authorized_action: stage only owned files and create the authorized local checkpoint commit.
- target_manifest: `EVID-CPU-MAIN-PRESETS-SOURCE-004`, patch SHA-256 `27fff031e3e02736c5feebbef90d495aa640e1c72ba70aad306ce6b4842fcb6c`.
- execution_target: `Windows 10.0.26200 x64; AMD Ryzen 7 9800X3D 8C/16T; Rust test/debug target; Node v22.20.0`.
- preserved_out_of_scope: `NOTICE.md`, `data/`, any other baseline dirty content.
- blockers: none.
- completed: main Vulkan control 아래 focus 6/8P·balanced 4/8P·safe 4/8P preset, advanced manual override, Efficiency/SchedulingClass 분리, hybrid 공유, 저코어와 live worker/service/UI/canonical records.
- checks: backend 127 PASS·3 ignored plus parity 2/2; desktop 8/8; Node 213/213; desktop locked check와 main UI smoke PASS; diff check PASS.
- limitations: actual Mabinogi performance and packaged helper forced-exit recovery `NOT_RUN`; repository-wide rustfmt check is pre-existing compact-format baseline 때문에 nonzero.
- findings: `ISSUE-PRESET-001`~`004` resolved; no unresolved mandatory findings.
- commit/push/deploy: local commit pending; push/deploy not authorized.
