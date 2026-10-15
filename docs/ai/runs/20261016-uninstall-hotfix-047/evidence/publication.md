# Public same-version replacement evidence

- evidence_id: `EVID-UNINSTALL-HOTFIX-047-PUBLICATION`
- producer: `cursor-agent-92251f36`
- created_at: `2026-10-15T18:26:29Z`
- run_id: `20261016-uninstall-hotfix-047`
- checkpoint_id: `CP-UNINSTALL-HOTFIX-047`
- source_target: `4ee58f36ceff917b135d80c809f439db39536ccb`
- release_target: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.7`
- public_download: `release/public-v0.4.7-uninstall-hotfix-2026-10-15T18-22-00Z`
- public_asset_manifest_sha256: `0ca31983e078d4e64a9ee148f5fb900a925783b10f2a8798b6f49a82f23eac39`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — uninstall hotfix publicly replaces 0.4.7 Rust assets`
- feature_map: `updated — revision 7`
- architecture_impact: `six Rust assets replaced in feed-safe order while preserving three Electron asset identities`
- architecture_contract: `updated — revision 31`

## Replacement

- replaced first: canonical setup, setup alias, canonical portable and portable alias.
- replaced last: `update.json` and `portable-update.json`.
- preserved: `latest.yml` asset ID `644166692`, migration EXE ID `644166693`, blockmap ID `644166694`; all retained original `2026-10-15T14:43:46Z` creation time.
- release notes: same-version clients may not auto-fetch; affected users are instructed to run the new setup manually as administrator.

## Public verification

- public assets: `9/9` names and SHA-256 match the approved replacement candidate.
- installer: `9,569,870 bytes`, SHA-256 `5d01a715a04d89bd2fbb531a2b3fb8fa9d85f0f0ee9e2cbd66aff17325ebff1c`.
- portable: `7,789,387 bytes`, SHA-256 `985c8a241760a90fe2fe456982490c4d15201b6340c0c3a97103191b655ac6e5`.
- update manifest: SHA-256 `2bdcdb6f2398db7e8459e6d6060d71225c263c32bd468466295cf03ffa0faed4`.
- portable manifest: SHA-256 `633e444399d594ee227cbabedc62179f03a02ca0a9ff0331ee6869bd6a611852`.
- `PASS` — both Ed25519 envelopes match public canonical executable URL, version, size and SHA-256.
- `PASS` — setup and portable aliases are byte-identical to canonical assets.
- `PASS` — public release remains non-draft and non-prerelease with nine assets.
- `PASS` — source commit is contained in pushed origin/main.

## Limits

- actual elevated installed-app upgrade/uninstall, scheduled task result and empty `C:\Program Files\Nogirem`: `NOT_RUN`.
- same-version automatic update rediscovery: `NOT_RUN` and not guaranteed; manual reinstall is the supported recovery.
