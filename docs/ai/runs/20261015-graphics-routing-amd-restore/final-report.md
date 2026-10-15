# Graphics routing and AMD restore final report

- record_id: `FINAL-GRAPHICS-RESTORE-001`
- run_id: `RUN-20261015-GRAPHICS-ROUTING-AMD-RESTORE`
- checkpoint_id: `CP-GRAPHICS-RESTORE-001`
- owner: `cursor-agent-92251f36`
- completed_at: `2026-10-15T13:55:00Z`
- disposition: `COMPLETED`
- base_commit: `061a23a4c7a6fc977ce8b3ec32e43c691ab8d1c0`
- implementation_commit: `d1b87085c360133eab49455348142efe94435f78`
- feature_impact: `FEAT-NOGIREM-GRAPHICS-SETTINGS — mixed GPU NVIDIA routing and AMD 3D optimization restore`
- feature_map: `updated — revision 2`
- architecture_impact: `NVIDIA-first adapter selection; AMD WMI-only status and explicit I3DSettings writer with verified rollback`
- architecture_contract: `updated — revision 29`

## Delivered

- RTX 4090 + AMD integrated mixed systems select the NVIDIA program profile without AMD WMI/helper access.
- AMD-only startup and refresh use WMI only and display four pending Radeon goals.
- Explicit AMD apply changes only VSync, Enhanced Sync, Anti-Lag and Chill through compatible-driver `I3DSettings`.
- Tuning/performance APIs and incompatible-driver initialization are prohibited and helper output must attest `tuningAccessed:false`.
- Apply snapshots every target GPU, verifies each target, restores and verifies snapshots on failure, and blocks game VSync unless helper goals are fully met.

## Gates and evidence

- automated: `PASS — graphics 6/6; backend 115 PASS·3 declared ignored; parity 2/2; Radeon Node 13/13; Node 211/211; desktop check; helper build; diff check; IDE diagnostics`.
- manually_exercised: `PASS — actual source status selected NVIDIA GeForce RTX 4090 and NVIDIA profile`.
- not_run: `AMD-only physical GPU apply and before/after undervolt observation; packaged UI; install; push; release; deployment`.
- review: `INDEPENDENT_REVIEW APPROVED — R3 exact six code hashes, ISSUE-GFX-R-001~003 RESOLVED, no must-fix`.
- feature_gate: `PASS for authorized source scope; hardware evidence gap disclosed`.
- architecture_gate: `PASS`.
- review_QA_gate: `PASS for applicable mandatory checks`.
- target_authorization_gate: `PASS — reviewed code committed; unrelated changes preserved; no external publication`.

## Remaining risk

- The source never invokes tuning APIs, but only an AMD-only hardware run can confirm a specific driver preserves an existing undervolt profile during normal ADLX initialization.
- Radeon and NVIDIA optimization still do not provide a persistent user-triggered restore workflow for previously configured 3D values after a successful optimization.
- commit_status: `local implementation commit created`.
- push_release_deployment_status: `NOT_RUN`.
