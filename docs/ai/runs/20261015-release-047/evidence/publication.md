# 0.4.7 public release evidence

- evidence_id: `EVID-RELEASE-047-PUBLICATION`
- producer: `cursor-agent-92251f36`
- created_at: `2026-10-15T14:44:43Z`
- run_id: `20261015-release-047`
- checkpoint_id: `CP-RELEASE-047`
- source_target: `62cf3ae48bb8a091bdb4d77a20c58f5cd7417f87`
- coordination_target_before_final_record: `2c548f6c8654f40b18adc27a3e0b29d04e9ef858`
- execution_target: `GitHub Release v0.4.7`
- release_url: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.7`
- feature_impact: `FEAT-NOGIREM-GRAPHICS-SETTINGS — restored mixed-GPU and AMD 3D behavior publicly distributed in 0.4.7`
- feature_map: `updated — revision 4 with public release evidence`
- architecture_impact: `signed package, Ed25519 update, Electron migration bridge and removed turbo asset distribution boundaries`
- architecture_contract: `no_change — revision 29 remains accurate`

## Targets and artifacts

- package: `release/dioxus-0.4.7-2026-10-15T14-25-48-115Z`
- candidate: `release/ready-v0.4.7-2026-10-15T14-41-46-964Z`
- public_download: `release/public-v0.4.7-2026-10-15T14-45-00Z`
- installer: `nogirem-dioxus-setup-0.4.7.exe`, `9,568,848 bytes`, SHA-256 `3b1f78b97b675a6e1ac06c4eec191e97c719b40b58dca9170171af1ef474b8ef`
- portable: `nogirem-dioxus-portable-0.4.7.exe`, `7,786,663 bytes`, SHA-256 `8721f700865b1ff5db719ff326da06feae62239e1654d6b4f731c5ecbc8c8ca4`
- update_manifest: SHA-256 `2ac5629535093a3e89cd4ac90f31e0568b9c5fe9c733c4d981584dc45bd47185`
- portable_manifest: SHA-256 `e5dfd63d1bddd8e8b01f23ec466f1536ef5081b9902990d592c243950d584623`
- electron_bridge: SHA-256 `63da5da24ed1aa89fb825ed8a0634d97a56114bd2eab9a4c0620fd48834ea366`
- latest_yml: SHA-256 `419a0e6c1bb6a9785df5d089ea2ba4188090dff807f940028a840e093a9bcc14`

## Checks

- `PASS` — canonical release assembly has 9 assets; setup·portable aliases are byte-identical.
- `PASS` — candidate update and portable manifests verify against the embedded Ed25519 public key.
- `PASS` — `latest.yml` keeps Electron migration clients on version `0.3.19` and the canonical migration asset.
- `PASS` — origin/main fast-forward push; annotated `v0.4.7` points to exact package source.
- `PASS` — GitHub Release is public, non-draft, non-prerelease and has 9 assets.
- `PASS 9/9` — every public re-download SHA-256 matches the local candidate.
- `PASS` — public download inputs reverify both Ed25519 payloads, alias identity and Electron routing.
- `PASS` — removed `turbo-key-helper` release asset is absent.
- `FAIL then PASS` — first candidate verifier expected the historical 10-asset count and failed at 9. Review showed the removed turbo helper was the tenth 0.4.6 asset; the corrected 0.4.7 contract requires 9 and the unchanged candidate passed.

## Limitations

- actual install, uninstall, portable launch, automatic update, game, AMD-only hardware and mixed-DPI scenarios are `NOT_RUN`.
- review_mode: `SELF_REVIEW`; no independent publication reviewer participated.
- no secrets or signing private key were recorded.
