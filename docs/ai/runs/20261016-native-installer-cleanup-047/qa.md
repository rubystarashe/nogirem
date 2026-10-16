# QA — native installer cleanup 0.4.7

- record_id: `QA-NATIVE-INSTALLER-047-001`
- run_id: `20261016-native-installer-cleanup-047`
- checkpoint_id: `CP-NATIVE-INSTALLER-CLEANUP-047`
- owner: `cursor-agent-92251f36`
- role_disclosure: `DEV-as-QA; independent code review participated separately`
- updated_at: `2026-10-16T06:07:23Z`
- qa_state: `COMPLETE_PASS for mandatory automated source, package and public checks; installed-app manual scenarios remain NOT_RUN`
- source_target: `commit 8d52c2f440f5256c9459d06e361188836bb968f2; reviewed implementation patch SHA-256 a097ac7a912c367663747f7661d76626e6c5ee92a4c91c190624b76a9174043b`
- execution_target: `Windows 10.0.26200 x64; source fixtures plus release/dioxus-0.4.7-2026-10-16T06-12-38-463Z`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — native install/uninstall cleanup`
- feature_map: `updated — revision 10 public replacement`
- architecture_impact: `native process identity, Task Scheduler CLI ownership query and NSIS adapter`
- architecture_contract: `updated — revision 34 public state`

## Executed scenarios

- `NATIVE-001` PASS: `cargo clippy --locked --manifest-path native/installer-cleanup/Cargo.toml --all-targets -- -D warnings`.
- `NATIVE-002` PASS: native helper unit tests `6/6`; duplicate options, Unicode ordinal comparison, path boundary, task XML ownership and reused parent PID rejection.
- `NATIVE-003` PASS: `node --test test/installer.test.mjs` `12/12`; current source is rebuilt before execution.
- `NATIVE-004` PASS: real temporary caller uninstaller remains alive.
- `NATIVE-005` PASS: stale updater and initial/late Unicode install-path processes terminate after one-second test timeout.
- `NATIVE-006` PASS: forged updater PID is rejected; actual parent updater is preserved and app rescan is skipped.
- `NATIVE-007` PASS: 0.4.1 parent updater without explicit argument is preserved.
- `NATIVE-008` PASS: duplicate install-directory injection is rejected without terminating the target process.
- `NATIVE-009` PASS: `npm test` `213/213`.
- `NATIVE-010` PASS: desktop locked check.
- `NATIVE-011` PASS: backend `115 PASS, 3 declared ignored`; parity `2/2`.
- `NATIVE-012` PASS: JavaScript syntax, `git diff --check`, IDE diagnostics.
- `NATIVE-013` PASS: NSIS `/WX` compile with embedded native helper; output `9,761,864 bytes`, below decimal 10 MB budget.
- `NATIVE-014` PASS: independent review final verdict `APPROVED`, all `ISSUE-NATIVE-001..004` resolved.

## Not run and limits

- `NATIVE-MANUAL-001` NOT_RUN: actual elevated 0.4.7 overwrite install under a restrictive PowerShell execution policy.
- `NATIVE-MANUAL-002` NOT_RUN: actual elevated uninstall, canonical task deletion and empty `C:\Program Files\Nogirem`.
- `NATIVE-MANUAL-003` NOT_RUN: forced Windows PID reuse and protected-process termination denial; source uses same-handle path/creation-time checks and fail-closed errors.
- `NATIVE-PACKAGE-001` PASS: signed package, Ed25519 envelopes, URL/size/SHA-256, no `.ps1`, required payload boundaries, aliases and native backend integration.
- `NATIVE-PUBLIC-001` PASS: executable assets replaced before manifests; public `9/9` candidate hash match, Ed25519, aliases, release state, Electron asset IDs and pushed source provenance verified.
