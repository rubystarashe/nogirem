# QA

- record_id: `QA-UNINSTALL-HOTFIX-047`
- run_id: `20261016-uninstall-hotfix-047`
- checkpoint_id: `CP-UNINSTALL-HOTFIX-047`
- owner: `cursor-agent-92251f36 acting as DEV-as-QA`
- updated_at: `2026-10-15T18:10:01Z`
- source_target: `base e975a26865e1481984a092d93fa560d5abf2fa27 + owned 8-file manifest 77ad85838c18dcf54ef417494b9bece98e695d76291e78f1e4041375167c7c9e`
- execution_target: `Windows 10.0.26200 x64 source checkout and temporary real-process fixtures`
- qa_state: `COMPLETE_PASS for source checkpoint`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — uninstall process stop, scheduled task ownership and retired asset cleanup`
- feature_map: `updated — revision 6`
- architecture_impact: `process-stop fail-closed, canonical task best-effort cleanup and exact retired asset deletion`
- architecture_contract: `updated — revision 30`

## Scenario results

- `UPD-004`: `PASS` — real temporary `Uninstall.exe` process is excluded from termination.
- `UPD-005`: `PASS` — stale updater and late-created install process are rescanned and terminated.
- `UPD-006`: `PASS` — trusted and legacy updater parent paths remain compatible.
- `UPD-008`: `PASS` — twelve retired app-owned files are exact-listed; wildcard, recursive deletion, AppData and traversal are rejected.
- `UPD-009`: `PASS` — canonical root task name, one action, exact executable and case-sensitive `--startup-tray` are all required; task cleanup failure is nonblocking while process failures remain nonzero.
- `NSIS-ERROR-DETAIL`: `PASS` — captured stop-script output is printed and included in blocking install/uninstall errors.

## Executed checks

- targeted installer, prohibited image recognition and turbo tests: `27/27 PASS`.
- installer after review fixes: `11/11 PASS`.
- full Node: `212/212 PASS`.
- backend: `115 PASS`, `3 declared ignored`; parity `2/2 PASS`.
- desktop: `cargo check --locked --manifest-path desktop/Cargo.toml PASS`.
- PowerShell parse: `PASS`; the first shell wrapper lost `$null` to interpolation and printed a command error despite exit 0, corrected command then passed.
- `git diff --check`: `PASS`.
- IDE diagnostics: `0`.

## Limitations

- actual elevated installed NSIS upgrade/uninstall and final empty `C:\Program Files\Nogirem` confirmation: `NOT_RUN`.
- signed package generation, same-version update manifests and public asset replacement: `NOT_RUN` at this checkpoint.
- no production/private data or credentials were captured.
