# Graphics routing and AMD restore coordination

- record_id: `COORD-GRAPHICS-RESTORE-001`
- run_id: `RUN-20261015-GRAPHICS-ROUTING-AMD-RESTORE`
- checkpoint_id: `CP-GRAPHICS-RESTORE-001`
- owner: `cursor-agent-92251f36`
- updated_at: `2026-10-15T13:41:00Z`
- exact_target: `implementation commit d1b87085c360133eab49455348142efe94435f78`
- feature_impact: `FEAT-NOGIREM-GRAPHICS-SETTINGS — routing and AMD optimization`
- feature_map: `updated — revision 2`
- architecture_impact: `GPU vendor adapter and Radeon API ownership`
- architecture_contract: `updated — revision 29`

## 상태와 소유권

- Coordinator/DEV/QA/docs: `cursor-agent-92251f36`.
- Independent Reviewer: `generalPurpose adf951c0-1cb0-4463-8c6b-2751a0e957fe`.
- active_checkpoint: `CP-GRAPHICS-RESTORE-001`.
- DEV: `DONE`.
- QA: `COMPLETE_PASS for available automation`.
- Review: `APPROVED — R3, ISSUE-GFX-R-001~003 RESOLVED`.
- owned_files: `.handoff/cursor.md`, `VERSION_HISTORY.md`, `desktop/backend/src/graphics.rs`, `desktop/src/ui.rs`, `native/radeon-helper/main.cpp`, `native/radeon-helper/bin/radeon-helper.exe`, `src/radeon.mjs`, `test/radeon.test.mjs`, canonical wiki files, this run directory.
- out_of_scope_preserved: user `NOTICE.md`, untracked `data/`, `native/installer-cleanup/`.
- cross_repository_dependencies: `none`.
- contract_authority: `nogirem maintainer`; current implementation follows canonical GPU boundary revision 29.
- open_blocker: `none for authorized source scope`.
- manual_follow_up_owner: AMD 단독 GPU hardware owner가 apply 전후 tuning 유지 확인.
- publishing_status: package/push/release/deployment unauthorized and NOT_RUN.
