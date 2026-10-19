# Source manifest — main CPU presets

- evidence_id: `EVID-CPU-MAIN-PRESETS-SOURCE-004`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-MAIN-PRESETS-004`
- producer: `cursor-agent-92251f36`
- created_at: `2026-10-19T06:52:00Z`
- repository/branch: `C:\Users\User\Documents\GitHub\nogirem / main`
- base_commit: `17dee9063db8c9926d70a1f9eab5a4955b7c47d8`
- index_state: `no staged files at manifest creation`
- worktree_state: `implementation and canonical files below; run records tracked separately`
- out_of_scope_dirty: `NOTICE.md modified; data/ untracked; preserved and excluded`
- patch_sha256: `27fff031e3e02736c5feebbef90d495aa640e1c72ba70aad306ce6b4842fcb6c`
- patch_procedure: `cmd redirection of git diff --binary over listed source/canonical files, then PowerShell Get-FileHash SHA256`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — main three presets and advanced manual count`
- feature_map: `updated — revision 5`
- architecture_impact: `preset defaults, topology classification, settings/worker IPC and main UI placement`
- architecture_contract: `updated — revision 38`

## File SHA-256

```text
9317fd935f636214e560f3f111171a51649aeeda696b1b30b947d3946777c98f  .handoff/cursor.md
71b4efa4fd5ba57c19118a30ae88c0f4342e8bd3aa642a7d3ea094c204a74e88  desktop/backend/src/affinity.rs
a4b6e9f53b5b8a06c0275aa7a662cdcec0e461d6cefc4488bf7c55ccf5c60c15  desktop/backend/src/affinity_worker.rs
bdebe84cebd79366d1603beb97221bd81bfef2c7d124b845b9a00c35271ffa5d  desktop/backend/src/bin/nogirem-cli.rs
6667894781903c3e35e1c1e1625dbf711f37c50fe21e822b4798fc9d6a9e2374  desktop/backend/src/boost.rs
182a109996d629c36191d08b0d1e88034f398ca4de74c8c8aa3aeabf50a71d68  desktop/backend/src/service-contract.json
1e91288d754a19465f964ed9521dac947e2b79742241e29cc9cc7e38dafeede2  desktop/backend/src/service.rs
f8aa4ba1f5429bf92ee6452757de2862ab8a1e0781bad7b0ad55a591144cacb4  desktop/backend/src/service_fixture.rs
cbaae405bae78985c6c81484700a8acf7bf0c169e25b751c1527d20090504cf8  desktop/backend/src/settings.rs
5e99d9076bdcda4a05f38c4c7cefcb63ce7ef3ccbc57889d863e68fb2519111d  desktop/backend/src/topology.rs
e76cdd2c4c25053bfdf3f54e3afd40c3f81c1f265fb0bf6aa374f50b7dd85baa  desktop/backend/tests/legacy-parity.rs
164992cd46c34d3348dd882b4a930d350e3a43c3daac49f5f84e8069f493001d  desktop/src/native.rs
fe3e6fef654f70edd390f16585e63110c7a8c0a139bb27ad79d4902a30a80f58  desktop/src/smoke.rs
142a7d559af1345df468699ea9f6ee415e4b4fcea74f3f7ac57c4946aaf81a28  desktop/src/ui.rs
dab81f01d2132b4dd66956ce76bf8cca036f210e9447d2dfe7dbc9d892339756  docs/ai/wiki/agent-friendly-architecture.md
6d8f9692edd9947f2fdfec8c2bc2fa83f64833646b26c69a535db036ba3bc42c  docs/ai/wiki/feature-map.md
c3e5a2574d930da8548c62e6ac3d4570b4e3376d1998c39294cec00d37ca7d17  web/styles.css
```

No deletions, renames, symlinks, submodules or binary source files are in this target. Run records are append-only coordination evidence and are staged explicitly with the source but excluded from the self-referential source patch hash.

## UI evidence

- smoke report SHA-256: `42a17e31f56cf991b34b34663febae063b979f02ed42a1c7ec3c92c1c33acca8`
- screenshot SHA-256: `d300d7048104e90f808a3b5b9fbfb323f879f2c7ab59ae64b12066894343c87a`
- environment: `Windows 10.0.26200 x64, debug Dioxus/WebView2 fixture`
- result: `PASS`; report `passed:true`, screenshot shows presets directly below Vulkan control.
- retention: artifacts are stored in the current agent store and are not committed; no private production or game data is present.
