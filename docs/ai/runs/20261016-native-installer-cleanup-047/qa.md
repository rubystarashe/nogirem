# QA — native installer cleanup 0.4.7

- record_id: `QA-NATIVE-INSTALLER-047-001`
- run_id: `20261016-native-installer-cleanup-047`
- checkpoint_id: `CP-NATIVE-INSTALLER-CLEANUP-047`
- owner: `cursor-agent-92251f36`
- role_disclosure: `DEV-as-QA; independent code review participated separately`
- updated_at: `2026-10-16T06:07:23Z`
- qa_state: `COMPLETE_PASS for automated source checks; installed-app manual scenarios remain NOT_RUN`
- source_target: `base 880ab9f533f5844f1ffcfde67de9352ba00e4ef9 + staged implementation patch SHA-256 a097ac7a912c367663747f7661d76626e6c5ee92a4c91c190624b76a9174043b`
- execution_target: `Windows 10.0.26200 x64; Rust host and x86_64-pc-windows-msvc release helper; temporary real cmd.exe process fixtures`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — native install/uninstall cleanup`
- feature_map: `blocked — canonical update follows source commit and signed package`
- architecture_impact: `native process identity, Task Scheduler CLI ownership query and NSIS adapter`
- architecture_contract: `blocked — canonical update follows source commit`

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
- `NATIVE-PUBLIC-001` NOT_RUN: signed package and GitHub same-version asset replacement belong to the next authorized steps.
