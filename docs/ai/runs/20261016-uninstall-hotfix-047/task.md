# 0.4.7 제거 hotfix와 동일 버전 대치

- record_id: `RUN-20261016-UNINSTALL-HOTFIX-047`
- run_id: `20261016-uninstall-hotfix-047`
- checkpoint_id: `CP-UNINSTALL-HOTFIX-047`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-15T17:53:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `e975a26865e1481984a092d93fa560d5abf2fa27`
- public_release: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.7`
- request: `수정하고 재배포해 같은버전 대치로`
- authorization: `0.4.7 source fix, local commits, origin/main push and existing v0.4.7 asset replacement`
- roles: `Coordinator=DEV=QA=Documentation maintainer cursor-agent-92251f36; independent reviewer generalPurpose agent 0051f39a-3f3d-46c3-978f-d3bfc2e13bec`
- dev_state: `DONE`
- qa_state: `COMPLETE_PASS`
- review_verdict: `APPROVED after documentation freshness rereview`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — 실행 프로세스가 없는데 제거가 차단되고 retired package files가 install directory에 남는 결함`
- feature_map: `blocked — exact fix and verification evidence pending`
- architecture_impact: `installer stop-script failure semantics, scheduled-task cleanup ownership and exact retired-asset deletion boundary`
- architecture_contract: `blocked — cleanup path and enforcement evidence update pending`

## Diagnosis and scope

- observed_target: installed `C:\Program Files\Nogirem`, registered version `0.4.7`.
- observed: install/updater process `0`; startup task `Mabinogi Rem Booster Startup` remains; uninstall dialog maps any stop-script nonzero exit to a misleading running-app message.
- observed: current install contains seven retired payload files absent from 0.4.7 app payload plus `Uninstall.exe`. Current generated uninstall manifest does not own those retired paths, so final non-recursive `RMDir "$INSTDIR"` can leave the directory.
- objective: make task cleanup failure non-blocking after a precise best-effort attempt, preserve real process-stop failures, remove only enumerated retired app-owned paths during upgrade and uninstall, expose stop-script failure detail, and replace signed 0.4.7 public assets.
- exclusions: recursive deletion of the whole install directory, deletion of AppData settings/recordings/snapshots, user-created unknown files, actual local installed-app removal without a separate explicit action.
- risk: `HIGH — privileged uninstall behavior and already-public same-version signed assets change; update cache identity and public download hashes must be revalidated`.
- unrelated_preserved: user `NOTICE.md`, untracked `data/`, `native/installer-cleanup/`.

## Plan and stop condition

1. Add a narrow retired-assets cleanup include with exact files and empty-directory removal only.
2. Make startup task cleanup best-effort and show captured stop-script reasons for actual blocking failures.
3. Add previous-install residual and failure-message regression tests; run installer, Node, backend and desktop checks.
4. Obtain independent review, resolve mandatory findings, update feature/architecture records and commit exact source.
5. Rebuild signed 0.4.7 setup/portable and verify manifests, payload and installer cleanup inputs.
6. Replace all mutable v0.4.7 Rust assets and aliases, preserve Electron bridge assets, publicly re-download and verify, then push final records.

- stop_condition: no unresolved mandatory finding; exact retired paths are removed without broad recursive deletion; actual stop failures expose useful reasons; signed package and public same-version replacement hashes, signatures and routing pass.

## Source checkpoint

- target: `base e975a26865e1481984a092d93fa560d5abf2fa27 + owned 8-file manifest 77ad85838c18dcf54ef417494b9bece98e695d76291e78f1e4041375167c7c9e`
- implementation: canonical task cleanup requires root path, exact name, one action, exact executable and case-sensitive `--startup-tray`; task cleanup failure is nonblocking.
- implementation: actual app/updater process failures remain blocking and NSIS displays captured stop-script details.
- implementation: upgrade and uninstall exact-delete twelve retired app-owned paths and only remove known empty directories.
- validation: installer `11/11`, full Node `212/212`, backend `115 PASS·3 ignored` plus parity `2/2`, desktop check, PowerShell parse, diff check and lint `0`.
- review: `INDEPENDENT_REVIEW APPROVED`, two must-fix findings resolved, unresolved `0`.
- remaining_checkpoint: source commit, signed package, package verification, same-version public replacement and public re-download verification.

## Package checkpoint

- source_target: `4ee58f36ceff917b135d80c809f439db39536ccb`
- package_target: `release/dioxus-0.4.7-2026-10-15T18-11-53-739Z`
- installer: `9,569,870 bytes`, SHA-256 `5d01a715a04d89bd2fbb531a2b3fb8fa9d85f0f0ee9e2cbd66aff17325ebff1c`
- portable: `7,789,387 bytes`, SHA-256 `985c8a241760a90fe2fe456982490c4d15201b6340c0c3a97103191b655ac6e5`
- result: `PASS — signed manifests, native contract, NSIS /WX, stop-script source identity, exact cleanup source, payload boundary and native integration 3/3`
- remaining_checkpoint: package publication review, source push, v0.4.7 asset replacement, public re-download verification and final records.

## Same-version replacement plan

- replace only six changed Rust assets: canonical setup, setup alias, canonical portable, portable alias, `update.json`, `portable-update.json`.
- preserve byte-identical Electron assets and their GitHub asset identities: `latest.yml`, migration EXE and blockmap.
- replace four executable assets first and both signed manifests last so feeds never advertise a payload before its matching bytes are available.
- same-version clients may not detect the replacement through semver; release notes and final report must instruct affected 0.4.7 users to manually reinstall the new setup asset.

## Final result

- status: `COMPLETED`
- completed_at: `2026-10-15T18:26:29Z`
- public_release: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.7`
- public_asset_manifest_sha256: `0ca31983e078d4e64a9ee148f5fb900a925783b10f2a8798b6f49a82f23eac39`
- replacement: `PASS — four Rust executables/aliases first, two signed manifests last; Electron asset IDs 644166692/693/694 preserved`
- public_validation: `PASS — 9/9 hashes, Ed25519 payloads, aliases, notes and source containment`
- review: `INDEPENDENT_REVIEW; implementation issues 2 resolved, publication scope issue 1 resolved, documentation freshness issue resolved after rereview`
- feature_gate: `PASS — revision 7`
- architecture_gate: `PASS — revision 31`
- review_qa_gate: `PASS for automated source, package and public replacement scope`
- target_authorization_gate: `PASS`
- manual_QA: `NOT_RUN — actual elevated install/uninstall, task deletion result, empty install directory and same-version automatic rediscovery`
