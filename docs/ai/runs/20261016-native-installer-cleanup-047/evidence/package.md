# Signed 0.4.7 native cleanup package evidence

- evidence_id: `EVID-NATIVE-INSTALLER-047-PACKAGE`
- producer: `cursor-agent-92251f36`
- created_at: `2026-10-16T06:16:04Z`
- run_id: `20261016-native-installer-cleanup-047`
- checkpoint_id: `CP-NATIVE-INSTALLER-CLEANUP-047`
- source_target: `faee6cf2492b8247a11b4a787f3c0f8cf710b060`
- package_input_override: `user NOTICE.md SHA-256 af1eb83b4ad8681314cf3123bf70f869f4d910434b401954a7c6825a8625f1f2`
- execution_target: `Windows 10.0.26200 x64; release/dioxus-0.4.7-2026-10-16T06-12-38-463Z`
- release_candidate: `release/ready-v0.4.7-2026-10-16T06-16-04-065Z`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — native cleanup packaged for same-version replacement`
- feature_map: `updated — revision 9 signed package evidence`
- architecture_impact: `compiled native helper embedded in NSIS without end-user PowerShell`
- architecture_contract: `updated — revision 33 signed package boundary`

## Artifacts

- installer: `nogirem-dioxus-setup-0.4.7.exe`, `9,761,105 bytes`, SHA-256 `23039c6377e17df4976914af7b78392d2ff1975efb76d0c3c8cec14640ca4d6c`.
- portable: `nogirem-dioxus-portable-0.4.7.exe`, `7,787,597 bytes`, SHA-256 `4b1646250dbcfc2841ed46f23a97a94488e08caa044ca4794a7e96354dc349f0`.
- native cleanup helper build: SHA-256 `a8daee085f2b13ae23cc44bb8a7bf1983e3248e8875cf350bbcd01320f9fd570`.
- update manifest: SHA-256 `65f83d33d68a831f607a31e28cfef44e2d73141dc8e3b872b2557482db427a03`.
- portable manifest: SHA-256 `265974969014a357863dcf568b621e14df36b1847a6e0328158470bdf778e9eb`.
- portable_cache_id: `aa05b723199bc6ddd706a49cd0b58e9e318d1e7c9600db08d8146b0f562bbb3e`.

## Checks and limits

- `npm run package:win`: `PASS` — current-source helper build, Rust release build, packaged service contract, WebView2 signature, NSIS `/WX`, decimal 10 MB budget and manifest signing.
- independent manifest verifier: `PASS` — both Ed25519 envelopes, canonical URLs, version, size and SHA-256.
- package boundary: `PASS` — no `.ps1`, stop script, retired overlay files, turbo terms, source-only images or Node backend payload; packaged NOTICE exact hash.
- aliases: `PASS` — setup and portable public aliases are byte-identical to canonical executable assets.
- native backend integration: `3/3 PASS`.
- helper error path: `PASS` — missing mandatory arguments returns exit code 1 and Korean diagnostic.
- verifier correction: an initial local one-liner incorrectly required a nonexistent manifest `filename` field and failed; corrected canonical URL validation passed. This was a test-harness error, not an artifact failure.
- actual elevated install/uninstall under restricted execution policy: `NOT_RUN`.
- public replacement: `NOT_RUN` at this checkpoint.
