# REV-NATIVE-INSTALLER-047-001

- run_id: `20261016-native-installer-cleanup-047`
- checkpoint_id: `CP-NATIVE-INSTALLER-CLEANUP-047`
- reviewer: `generalPurpose agent fa4f497b-eb4b-45f9-871b-bb8833541e8b`
- review_mode: `INDEPENDENT_REVIEW`
- created_at: `2026-10-16T05:51:00Z`
- final_updated_at: `2026-10-16T06:03:00Z`
- base: `880ab9f533f5844f1ffcfde67de9352ba00e4ef9`
- implementation_target: `staged implementation patch SHA-256 a097ac7a912c367663747f7661d76626e6c5ee92a4c91c190624b76a9174043b`
- manifest_procedure: `explicit implementation paths staged; PowerShell UTF-8 bytes of git diff --cached --binary hashed with SHA-256`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — installer and uninstaller process cleanup`
- feature_map: `no_change — review record; canonical update follows source commit`
- architecture_impact: `NSIS cleanup adapter changes from extracted PowerShell to native Rust/Win32 helper`
- architecture_contract: `no_change — review record; canonical update follows source commit`

## Request and acknowledgment

- requested: inspect the actual diff and surrounding updater/service code for caller exclusion, updater parent/path trust, legacy compatibility, PID races, Unicode paths, task ownership, fail-closed termination, package reproducibility, tests and complete PowerShell removal.
- reviewer_acknowledgment: target accessible; `NOTICE.md` and `data/` excluded; reviewer made no file changes or commits.

## Round 1 findings

- `ISSUE-NATIVE-001`, HIGH, must_fix: untrusted `/NOGIREMUPDATE=` text could append helper options and duplicate singleton options were accepted.
- `ISSUE-NATIVE-002`, HIGH, must_fix: process path was checked in one handle then termination reopened by PID without creation-time identity or API failure checks.
- `ISSUE-NATIVE-003`, MEDIUM, must_fix: lossy/ASCII path comparison and UTF-8 helper output through NSIS ACP could miss or garble non-ASCII cases.
- `ISSUE-NATIVE-004`, MEDIUM, must_fix: external Cargo target overrides could copy a stale binary and tests depended on a previously built ignored executable.
- verdict: `CHANGES_REQUESTED`

## DEV response and round 2

- `ISSUE-NATIVE-001`: fixed with canonical positive decimal NSIS validation, fixed argument branches and duplicate/zero option rejection. `RESOLVED`.
- `ISSUE-NATIVE-003`: fixed with UTF-16 `OsString`, `CompareStringOrdinal`, lexical/canonical prefixes and fixed NSIS Korean error text. `RESOLVED`.
- `ISSUE-NATIVE-004`: fixed with explicit `x86_64-pc-windows-msvc`, fixed `CARGO_TARGET_DIR`, stale output removal and test-time source build. `RESOLVED`.
- `ISSUE-NATIVE-002`: reviewer found identity-query failure still fail-open and preserved updater still PID-only. `OPEN`.
- verdict: `CHANGES_REQUESTED`

## DEV response and final rereview

- `ISSUE-NATIVE-002`: identity query now returns `Result`; a failed query succeeds only when the same handle is already signaled. The trusted updater must have been created no later than the caller and preserved exclusion uses PID, creation time and path identity. `RESOLVED`.
- new findings: `none`
- final_verdict: `APPROVED`
- stale: `false for implementation patch a097ac7a912c367663747f7661d76626e6c5ee92a4c91c190624b76a9174043b`
- limitations: actual NSIS install/uninstall, scheduled-task deletion, denied termination and forced PID reuse were not manually executed; reviewer independently ran format, syntax and diff checks but relied on DEV execution records for clippy, unit, integration and NSIS compile results.
