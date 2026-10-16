# Public same-version native cleanup replacement evidence

- evidence_id: `EVID-NATIVE-INSTALLER-047-PUBLICATION`
- producer: `cursor-agent-92251f36`
- created_at: `2026-10-16T06:21:00Z`
- run_id: `20261016-native-installer-cleanup-047`
- checkpoint_id: `CP-NATIVE-INSTALLER-CLEANUP-047`
- package_source_target: `faee6cf2492b8247a11b4a787f3c0f8cf710b060`
- publication_record_target: `26ba2c0686a0d2d7a1a1213a4698877d0b0f35b0 pushed to origin/main`
- release_target: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.7`
- public_download: `release/public-v0.4.7-native-cleanup-2026-10-16T06-20-00Z`
- public_asset_manifest_sha256: `0636b672540c3f3acad2701cb7a4fa64b99554e0ed05cceeb2e00d2430851b1c`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — native cleanup publicly replaces PowerShell-based 0.4.7 assets`
- feature_map: `updated — revision 10 public replacement`
- architecture_impact: `six Rust assets replaced in feed-safe order while preserving three Electron asset identities`
- architecture_contract: `updated — revision 34 public boundary`

## Replacement

- replaced first at `2026-10-16T06:19:38Z`: canonical setup, setup alias, canonical portable and portable alias.
- replaced last at `2026-10-16T06:19:52Z`: `update.json` and `portable-update.json`.
- preserved Electron asset IDs: `latest.yml=RA_kwDOUPAUrs4mZTQk`, migration EXE `RA_kwDOUPAUrs4mZTQl`, blockmap `RA_kwDOUPAUrs4mZTQm`.
- release remains non-draft and non-prerelease with exactly nine assets.
- release notes now identify the removed PowerShell-script dependency and same-version manual overwrite recovery.

## Public verification

- public assets: `9/9` names, sizes and SHA-256 are byte-identical to the approved candidate.
- installer and alias: `9,761,105 bytes`, SHA-256 `23039c6377e17df4976914af7b78392d2ff1975efb76d0c3c8cec14640ca4d6c`.
- portable and alias: `7,787,597 bytes`, SHA-256 `4b1646250dbcfc2841ed46f23a97a94488e08caa044ca4794a7e96354dc349f0`.
- update manifest: SHA-256 `65f83d33d68a831f607a31e28cfef44e2d73141dc8e3b872b2557482db427a03`.
- portable manifest: SHA-256 `265974969014a357863dcf568b621e14df36b1847a6e0328158470bdf778e9eb`.
- `PASS` — both Ed25519 envelopes match public canonical executable URL, version, size and SHA-256.
- `PASS` — setup and portable aliases are byte-identical to canonical assets.
- `PASS` — package source and publication evidence checkpoint are contained in pushed origin/main.

## Limits

- actual elevated overwrite/uninstall, canonical scheduled-task deletion and empty `C:\Program Files\Nogirem`: `NOT_RUN`.
- same-version automatic update rediscovery: `NOT_RUN` and not guaranteed; manual administrator overwrite remains the supported recovery for affected users.
