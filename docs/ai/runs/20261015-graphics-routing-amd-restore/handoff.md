# Graphics routing and AMD restore handoff

- record_id: `HANDOFF-GRAPHICS-RESTORE-001`
- run_id: `RUN-20261015-GRAPHICS-ROUTING-AMD-RESTORE`
- checkpoint_id: `CP-GRAPHICS-RESTORE-001`
- owner: `cursor-agent-92251f36`
- updated_at: `2026-10-15T13:55:00Z`
- status: `COMPLETED`
- DEV_state: `DONE`
- QA_state: `COMPLETE_PASS for applicable automation`
- review_state: `APPROVED — REV-GRAPHICS-RESTORE-001 R3`
- source_target: `d1b87085c360133eab49455348142efe94435f78`
- execution_target: `Windows 10.0.26200 x64 source debug, RTX 4090 + AMD Radeon(TM) Graphics`
- feature_impact: `FEAT-NOGIREM-GRAPHICS-SETTINGS — mixed GPU routing and AMD optimization restore`
- feature_map: `updated — revision 2`
- architecture_impact: `NVIDIA-first routing and AMD WMI/status versus explicit I3DSettings writer`
- architecture_contract: `updated — revision 29`

## Resume state

- delivered: NVIDIA-first selection, AMD WMI-only status, explicit Radeon 3D apply, tuning API prohibition, snapshot rollback with value verification, game VSync pre-write guards, UI and compatibility updates.
- verification: graphics 6/6, backend 115 PASS·3 ignored, parity 2/2, Radeon Node 13/13, Node 211/211, desktop check, helper build, actual RTX 4090 status, diff/lint and independent review PASS.
- exact reviewed hashes: see `task.md` and `reviews/REV-GRAPHICS-RESTORE-001.md`.
- pending_nonblocking: AMD-only hardware apply and before/after undervolt verification.
- package_push_release_deploy: `NOT_RUN`.
- unrelated_preserved: user `NOTICE.md`, untracked `data/`, `native/installer-cleanup/`.
- next_authorized_action: none. A future hardware-QA checkpoint may run the AMD-only scenario; packaging or publishing requires separate authorization.
