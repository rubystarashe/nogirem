# Final report — native installer cleanup 0.4.7

- status: `COMPLETED`
- run_id: `20261016-native-installer-cleanup-047`
- checkpoint_id: `CP-NATIVE-INSTALLER-CLEANUP-047`
- owner: `cursor-agent-92251f36`
- completed_at: `2026-10-16T06:21:00Z`
- requested_scope: `replace execution-policy-blocked PowerShell cleanup, repackage 0.4.7, replace the same public version and push`
- delivered_scope: `native helper source/build/tests, NSIS integration, canonical docs, signed package, GitHub six-asset replacement, public verification and source pushes`
- implementation_commit: `8d52c2f440f5256c9459d06e361188836bb968f2`
- package_source_commit: `faee6cf2492b8247a11b4a787f3c0f8cf710b060`
- pre_publication_record_commit: `26ba2c0686a0d2d7a1a1213a4698877d0b0f35b0`
- public_execution_target: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.7`
- public_asset_manifest_sha256: `0636b672540c3f3acad2701cb7a4fa64b99554e0ed05cceeb2e00d2430851b1c`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — install/uninstall cleanup no longer depends on PowerShell script policy`
- feature_map: `updated — revision 10`
- architecture_impact: `NSIS embeds a Rust/Win32 helper with process handle identity and exact task ownership`
- architecture_contract: `updated — revision 34`

## Delivered behavior

- NSIS no longer extracts or executes `stop-installed-app.ps1`; the file was removed.
- updater PID input is canonical positive decimal and helper singleton arguments reject duplicates.
- process discovery is restricted to lexical/canonical UTF-16 install or protected updater paths.
- termination rechecks path and process creation time on the same handle, rejects PID reuse and fails closed on API or permission errors.
- exact trusted parent updater and 0.4.1 parent compatibility remain; unrelated stale updater processes are terminated.
- graceful close request, 30-second normal timeout, one-second quiet rescan and five-second forced-stop bound remain.
- only the canonical root startup task with one exact `Exec`, installed `nogirem.exe` and `--startup-tray` is deleted; task cleanup failure does not masquerade as process failure.

## Verification

- automated-test-passing: native unit `6/6`, installer real-process `12/12`, Node `213/213`, backend `115 PASS + 3 declared ignored`, parity `2/2`, native integration `3/3`, desktop locked check and strict clippy.
- package-passing: current-source helper build, packaged service, WebView2 signature, NSIS `/WX`, `<10,000,000` installer budget, Ed25519, URL/size/SHA-256, payload boundaries and aliases.
- independent_review: `APPROVED`; `ISSUE-NATIVE-001..004` resolved; approval applies to reviewed implementation patch `a097ac7a...043b`.
- public-passing: `9/9` public assets byte-identical to candidate; manifests published after executables; Electron bridge asset IDs preserved.
- installer: `9,761,105 bytes`, SHA-256 `23039c6377e17df4976914af7b78392d2ff1975efb76d0c3c8cec14640ca4d6c`.
- portable: `7,787,597 bytes`, SHA-256 `4b1646250dbcfc2841ed46f23a97a94488e08caa044ca4794a7e96354dc349f0`.

## Gates, limits and publication

- feature_gate: `PASS for requested source/package/public scope; lifecycle remains PARTIAL because installed-app manual and same-version rediscovery gaps remain`.
- architecture_gate: `PASS — no end-user PowerShell dependency, no unresolved review finding, no broad process kill or recursive delete`.
- review_and_qa_gate: `PASS for mandatory automated checks and public evidence`.
- target_and_authorization_gate: `PASS — explicit files committed, unrelated NOTICE/data preserved, source pushed, authorized release assets replaced`.
- manually_exercised: `temporary real Windows process fixtures only`.
- not_run: `actual elevated overwrite/uninstall under a restrictive execution policy, canonical scheduled-task observation, empty C:\Program Files\Nogirem and same-version automatic rediscovery`.
- recovery: affected existing 0.4.7 users should download and run `nogirem-setup-0.4.7.exe` as administrator because semver may not trigger a second automatic 0.4.7 download.
- commit_status: `local commits created and main pushed`.
- release_status: `GitHub v0.4.7 Rust assets replaced; tag was not moved`.
- deployment_status: `no production service deployment or activation; release notes updated`.
- unrelated_preserved: `user NOTICE.md and untracked data/`.
- remaining_actions: `none required for requested scope; optional installed-app manual QA needs a disposable or explicitly approved target`.
