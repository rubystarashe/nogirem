# Final report — CPU topology and priority

- record_id: `FINAL-CPU-TOPOLOGY-PRIORITY-001`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-TOPOLOGY-PRIORITY-001`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-19T05:35:00Z`
- disposition: `COMPLETED`
- source_target: `base 1502358cc77df6a35e3515cf5ec98ddd2f0737ab + EVID-CPU-TOPOLOGY-SOURCE-001`
- patch_sha256: `f0f3b3d3c3eb1a869ea39f19cc59b569c2cff913865fd98604b128dfe0c30c18`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — topology, non-overlapping affinity, advanced binding/priority and recovery`
- feature_map: `updated — canonical revision 31, feature revision 2`
- architecture_impact: `topology authority, same-handle process adapter, durable original-state ledger and narrow settings IPC`
- architecture_contract: `updated — revision 35`

## Delivered

- Combined CPU Set efficiency/scheduling data with GLPIEx physical-core, SMT, L3 and processor-group relationships.
- Allocated only the requested P-cores to the game mask, preferring one L3 domain, and assigned remaining P-cores plus E-cores to a disjoint background mask.
- Disabled CPU splitting with a visible reason on four-or-fewer physical cores, multiple processor groups or malformed/unrepresentable topology.
- Made game hard affinity opt-in and background `BELOW_NORMAL_PRIORITY_CLASS` a separate default-false option limited to the general background role.
- Added same-handle process identity, write-before-mutation original-state ledger, current-equals-applied restore guard, startup recovery and failed-entry retention.
- Added normalized persistence, live helper control, service channels, Dioxus controls and updated canonical records.

## Verification and review

- Automated: backend 124 passed·3 ignored plus integration parity 2/2; desktop 8/8; Node 213/213; desktop locked check passed.
- Manually exercised by test process: live Windows topology query and child-process priority apply/restore roundtrip.
- `NOT_RUN`: actual Mabinogi frame/input behavior and packaged helper forced-exit recovery UI.
- Strict Clippy: command executed but nonzero on repository baseline warnings introduced by Rust 1.98; no suppression or baseline expansion. The introduced constructor-argument warning was resolved.
- review_mode: `SELF_REVIEW`; verdict `APPROVED`; `ISSUE-CPU-001`~`003` resolved; no independent reviewer.

## Gates and remaining risk

- Feature gate: `PASS` for implemented automated scope; actual game performance remains an explicit evidence gap.
- Architecture gate: `PASS`; no truncated multi-group mask, duplicate writer or broad exception.
- Review/QA gate: `PASS` with disclosed SELF_REVIEW and Clippy baseline limitation.
- Target/authorization gate: `PASS`; `NOTICE.md` and `data/` preserved and excluded.
- Commit: one local checkpoint commit contains the owned target and this report; its SHA is reported outside the containing commit to avoid a self-reference cycle.
- Push/deploy/release: not performed and not authorized.
