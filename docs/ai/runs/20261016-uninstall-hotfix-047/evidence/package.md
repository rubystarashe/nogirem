# Signed 0.4.7 hotfix package evidence

- evidence_id: `EVID-UNINSTALL-HOTFIX-047-PACKAGE`
- producer: `cursor-agent-92251f36`
- created_at: `2026-10-15T18:15:10Z`
- run_id: `20261016-uninstall-hotfix-047`
- checkpoint_id: `CP-UNINSTALL-HOTFIX-047`
- source_target: `4ee58f36ceff917b135d80c809f439db39536ccb`
- package_input_override: `user NOTICE.md SHA-256 af1eb83b4ad8681314cf3123bf70f869f4d910434b401954a7c6825a8625f1f2`
- execution_target: `Windows 10.0.26200 x64; release/dioxus-0.4.7-2026-10-15T18-11-53-739Z`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — uninstall hotfix packaged for same-version replacement`
- feature_map: `updated — revision 6`
- architecture_impact: `compiled NSIS cleanup ordering, stop script identity and signed update payload`
- architecture_contract: `updated — revision 30`

## Artifacts

- installer: `nogirem-dioxus-setup-0.4.7.exe`, `9,569,870 bytes`, SHA-256 `5d01a715a04d89bd2fbb531a2b3fb8fa9d85f0f0ee9e2cbd66aff17325ebff1c`.
- portable: `nogirem-dioxus-portable-0.4.7.exe`, `7,789,387 bytes`, SHA-256 `985c8a241760a90fe2fe456982490c4d15201b6340c0c3a97103191b655ac6e5`.
- portable_cache_id: `9cfb2fce99d30f47338693960e1f7bd513898aa19171b86bfb721b446c6a0d22`.
- packaged_stop_script: SHA-256 `76becb6243998e4e9ef7359d679f6d6c968d5eecfef97b884f09150fc1bb6b25`, source identical.
- manifests: installer and portable Ed25519 signatures, version, size and SHA-256 `PASS`.

## Checks and limits

- `npm run package:win`: `PASS` — release build, native service contract, WebView2 signature, NSIS `/WX`, size budget and manifest self-verification.
- package verifier: `PASS` — signatures, payload files, forbidden retired assets, NOTICE and Radeon helper identity.
- retired cleanup source: `PASS` — included in install and uninstall; exact paths only, no wildcard or recursive deletion.
- native backend integration: `3/3 PASS`.
- runtimePackages: `[]`.
- actual install, uninstall and empty install directory: `NOT_RUN`.
- public same-version asset replacement: `NOT_RUN` at this checkpoint.
