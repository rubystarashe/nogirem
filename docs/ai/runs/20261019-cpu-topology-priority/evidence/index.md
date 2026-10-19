# Evidence index

- evidence_id: `EVID-CPU-TOPOLOGY-INDEX-001`
- producer: `cursor-agent-92251f36`
- created_at: `2026-10-19T05:35:00Z`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-TOPOLOGY-PRIORITY-001`
- source_target: `EVID-CPU-TOPOLOGY-SOURCE-001`
- execution_environment: `Windows 10.0.26200 x64; Rust test/debug target; Node v22.20.0`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — implementation and automated verification evidence`
- feature_map: `updated — revision 31`
- architecture_impact: `topology/process/ledger/IPC boundaries`
- architecture_contract: `updated — revision 35`

## Artifacts

- `source-manifest.md`: SHA-256 `b66cf89b216d312ffc0991df368227d8a84ee03dfaf0936b87abe464b9a1f33c`.
- command results: recorded in `../qa.md`; raw terminal output is transient and not committed.

## Provenance and limitations

File hashes were produced with `Get-FileHash -Algorithm SHA256`; the normalized patch hash procedure is recorded in the manifest. Results were not truncated in `qa.md`, but full command stdout is not retained. No private production data, credentials or game process data was captured.

## Follow-up EVID-CPU-MAIN-PRESETS-SOURCE-004

- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-MAIN-PRESETS-004`
- source_target: `base 17dee9063db8c9926d70a1f9eab5a4955b7c47d8 + source-manifest-main-presets.md`
- execution_environment: `Windows 10.0.26200 x64; AMD Ryzen 7 9800X3D 8C/16T; Rust/Node/Dioxus debug`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — main three presets and advanced manual count`
- feature_map: `updated — revision 5`
- architecture_impact: `preset defaults, topology classification, settings/worker IPC and main UI placement`
- architecture_contract: `updated — revision 38`
- artifacts: `source-manifest-main-presets.md`; external smoke report SHA-256 `42a17e31...acca8`; screenshot SHA-256 `d300d704...3c87a`.
- limitations: actual Mabinogi performance and packaged recovery remain `NOT_RUN`; external UI artifacts are current-agent-store retained, not committed.
