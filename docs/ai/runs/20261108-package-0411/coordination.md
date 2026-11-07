# Coordination

- record_id: `COORD-PACKAGE-0411`
- run_id: `20261108-package-0411`
- checkpoint_id: `CP-VERSION-0411`
- owner: `cursor-agent-92251f36 (Coordinator)`
- updated_at: `2026-11-07T17:33:00Z`
- source_target: `fix/dxvk-network-diagnostics-0411 @ 6aaa1c6e5beb5bfed1fda2e0466b165eadfc4d9f`
- feature_impact: `FEAT-NOGIREM-DXVK, FEAT-NOGIREM-NETWORK-FASTPING, FEAT-NOGIREM-UPDATE-LIFECYCLE`
- feature_map: `updated — pending package target`
- architecture_impact: `version/package/signing boundary`
- architecture_contract: `updated — pending package evidence`

## Roles and ownership

- Coordinator/DEV/QA/Reviewer/Feature owner/Architecture owner/Documentation maintainer: `cursor-agent-92251f36`
- review_mode: `SELF_REVIEW`
- owned_files: `VERSION_HISTORY.md`, package/Cargo version files and locks, this run, affected canonical metadata
- package_output_owner: `scripts/package-dioxus.mjs`
- concurrent_work: 기본 worktree의 source·binary·data 변경은 별도 소유이며 읽기 외에 건드리지 않는다.

## State

- DEV: `IN_PROGRESS`
- QA: `PLANNED`
- Review: `NOT_REVIEWED`
- Package: `NOT_STARTED`
- active_checkpoint: `CP-VERSION-0411`
- blockers: `none`
- external_actions: `push/tag/release/deploy NOT_AUTHORIZED`
