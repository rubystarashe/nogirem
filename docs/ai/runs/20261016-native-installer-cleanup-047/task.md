# 0.4.7 native installer cleanup hotfix

- record_id: `RUN-20261016-NATIVE-INSTALLER-CLEANUP-047`
- run_id: `20261016-native-installer-cleanup-047`
- checkpoint_id: `CP-NATIVE-INSTALLER-CLEANUP-047`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-16T05:41:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `880ab9f533f5844f1ffcfde67de9352ba00e4ef9`
- request: `바꿔그럼 ㅡㅡ`
- trigger: `public replacement installer exposes UnauthorizedAccess because Windows execution policy rejects extracted stop-installed-app.ps1`
- authorization: `replace installer PowerShell dependency, commit, push, rebuild signed 0.4.7 and replace public assets at the same version`
- roles: `Coordinator=DEV=QA=Documentation maintainer cursor-agent-92251f36; independent reviewer pending`
- dev_state: `READY_FOR_QA — source checkpoint commit pending`
- qa_state: `COMPLETE_PASS for automated source checks; installed-app manual scenarios NOT_RUN`
- review_verdict: `APPROVED — REV-NATIVE-INSTALLER-047-001`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — install/uninstall must work without PowerShell script execution policy`
- feature_map: `updated — revision 8 source checkpoint`
- architecture_impact: `NSIS cleanup adapter changes from extracted PowerShell script to packaged native executable`
- architecture_contract: `updated — revision 32 source checkpoint`

## Scope and risk

- objective: preserve canonical task ownership, stale updater cleanup, trusted updater parent validation, graceful app shutdown, late process rescan and fail-closed process termination in a native Windows helper.
- included: native helper source/build, NSIS integration, package build input, tests, signed package, six-asset same-version replacement, public re-download verification and source/final record push.
- excluded: lowering machine/user execution policy, `-ExecutionPolicy Bypass`, deleting AppData/user data, moving the existing tag, changing Electron migration assets.
- risk: `HIGH — privileged process termination and scheduled-task cleanup implementation changes, then already-public signed assets are replaced`.
- preexisting_artifacts: `native/installer-cleanup/target/**` existed untracked at intake; preserve and never stage. New authoritative source may reuse the directory while explicitly excluding target artifacts.
- unrelated_preserved: user `NOTICE.md`, untracked `data/`, preexisting native target artifacts.

## Plan and stop condition

1. Restore an authoritative Rust native helper under `native/installer-cleanup` with unit and real-process tests.
2. Replace NSIS `powershell.exe -File` calls with the helper and remove packaged script generation.
3. Verify task ownership, updater parent/path, graceful/forced stop, error output, build reproducibility and no PowerShell dependency.
4. Update canonical feature/architecture records and obtain independent review.
5. Commit/push exact source, rebuild signed 0.4.7, verify package, replace only six Rust assets in feed-safe order and publicly reverify.

- stop_condition: no installer/uninstaller PowerShell script execution; native helper and NSIS checks pass; independent review approves; public assets match signed package; actual installed-app manual limits remain explicit.

## Implementation checkpoint

- staged_implementation_patch_sha256: `a097ac7a912c367663747f7661d76626e6c5ee92a4c91c190624b76a9174043b`
- implemented: Rust/Win32 helper with process path and creation-time identity, updater parent trust, graceful request, bounded termination and owned startup-task cleanup.
- installer_contract: NSIS accepts only a canonical positive updater PID and embeds/runs `installer-cleanup.exe`; no end-user PowerShell or `.ps1` path remains.
- validation: native unit `6/6`, installer `12/12`, Node `213/213`, backend `115 PASS + 3 ignored`, parity `2/2`, desktop check, clippy strict, syntax/diff/IDE checks and NSIS `/WX` compile PASS.
- review: independent rounds resolved `ISSUE-NATIVE-001..004`; final `APPROVED`.
- next_authorized_action: create and push the source checkpoint, update canonical records to the immutable commit, then build and verify the signed 0.4.7 package before replacing public assets.
- implementation_commit: `8d52c2f440f5256c9459d06e361188836bb968f2`
- revised_next_authorized_action: commit and push canonical records, then build and verify signed 0.4.7 before replacing public assets.
