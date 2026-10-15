# 0.4.7 공개 배포

- record_id: `RUN-20261015-RELEASE-047`
- run_id: `20261015-release-047`
- checkpoint_id: `CP-RELEASE-047`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-15T14:40:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_target: `62cf3ae48bb8a091bdb4d77a20c58f5cd7417f87`
- coordination_target: `c061b765ac61095eb17865d332988f1d5b0aabd0 plus this run record`
- package_target: `release/dioxus-0.4.7-2026-10-15T14-25-48-115Z`
- request: `배포하고 푸쉬해`
- authorization: `origin/main push, annotated v0.4.7 tag and public GitHub Release explicitly authorized`
- roles: `Coordinator=DEV=QA=Reviewer=Documentation maintainer cursor-agent-92251f36; publication SELF_REVIEW`
- dev_state: `DONE`
- qa_state: `COMPLETE_PASS`
- review_verdict: `APPROVED`
- feature_impact: `FEAT-NOGIREM-GRAPHICS-SETTINGS — NVIDIA 우선 routing과 tuning API 미접근 AMD 3D 최적화를 0.4.7로 공개`
- feature_map: `no_change — revision 3 already records the exact signed package and remaining hardware gap`
- architecture_impact: `existing signed package, Ed25519 update, Electron migration bridge and GitHub Release boundaries executed`
- architecture_contract: `no_change — revision 29 already defines the affected distribution boundaries`

## Scope and risk

- objective: exact 0.4.7 package를 기존 Electron 0.3.19 migration bridge와 조립해 공개하고 source·records를 origin/main에 push한 뒤 공개 다운로드를 재검증한다.
- included: release asset assembly, candidate hash·routing verification, source push, annotated tag, public non-prerelease GitHub Release, public asset hash·signature·routing verification, final records and push.
- excluded: actual install, uninstall, portable launch, automatic update, game, AMD-only hardware and mixed-DPI manual QA.
- risk: `HIGH — signed privileged Windows binaries and update manifests become publicly downloadable and origin/main is mutated`.
- unrelated_preserved: uncommitted `NOTICE.md`, `data/`, `native/installer-cleanup/`.

## Plan and stop condition

1. Assemble the canonical 9 assets, excluding the removed turbo helper, and verify aliases, manifests and Electron routing.
2. Commit this release coordination record without unrelated user files.
3. Fast-forward push main and create annotated `v0.4.7` at exact package source `62cf3ae48bb8a091bdb4d77a20c58f5cd7417f87`.
4. Create a public non-draft, non-prerelease GitHub Release with all 10 assets.
5. Download all public assets and compare hashes, then re-run signed package and Electron routing verification.
6. Record evidence, final report and handoff; commit and push the final record.

- stop_condition: origin/main contains the release records, annotated v0.4.7 points to exact package source, public assets are 9/9 hash-identical and signed routing checks pass, or an exact blocker is recorded.

## Result

- status: `COMPLETED`
- completed_at: `2026-10-15T14:44:43Z`
- release_candidate: `release/ready-v0.4.7-2026-10-15T14-41-46-964Z`
- public_download: `release/public-v0.4.7-2026-10-15T14-45-00Z`
- release: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.7`
- source_push: `PASS — origin/main 2c548f6c8654f40b18adc27a3e0b29d04e9ef858 before final record`
- tag: `PASS — annotated v0.4.7 points to 62cf3ae48bb8a091bdb4d77a20c58f5cd7417f87`
- public_release: `PASS — public, non-draft, non-prerelease, Latest`
- public_validation: `PASS — 9/9 candidate hash match, Ed25519 manifests, installer·portable aliases and Electron 0.3.19 routing`
- removed_asset_boundary: `PASS — turbo-key helper is not published`
- review_mode: `SELF_REVIEW`
- review_disclosure: `same cursor agent assembled, published and verified the release; no independent publication reviewer participated`
- mandatory_findings: `none`
- manual_QA: `NOT_RUN — install, uninstall, portable launch, updater, game, AMD-only hardware and mixed DPI`
- feature_gate: `PASS — feature map revision 4 records public 0.4.7 evidence and hardware gap`
- architecture_gate: `PASS — signed package, migration bridge and removed-asset boundaries match revision 29`
- review_qa_gate: `PASS for automated publication scope`
- target_authorization_gate: `PASS — publication remained within explicit user authorization`
