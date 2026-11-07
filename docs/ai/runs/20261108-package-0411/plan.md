# Plan

- record_id: `PLAN-PACKAGE-0411`
- run_id: `20261108-package-0411`
- checkpoint_id: `CP-VERSION-0411`
- owner: `cursor-agent-92251f36 (DEV)`
- created_at: `2026-11-07T17:33:00Z`
- status: `completed`
- feature_impact: `FEAT-NOGIREM-DXVK, FEAT-NOGIREM-NETWORK-FASTPING, FEAT-NOGIREM-UPDATE-LIFECYCLE`
- feature_map: `updated — package completion 시 exact target 반영`
- architecture_impact: `signed package assembly와 version identity`
- architecture_contract: `updated — package enforcement result 반영`

## Checkpoints

1. `CP-VERSION-0411` (`completed`): 0.4.11 history·package·Cargo·lock identity, source `545ec1767e03745bc598cc90c48f38ed484db987`
2. `CP-QA-0411` (`completed`): backend·parity, desktop test/check, Node, native integration, version checks, SELF_REVIEW
3. `CP-PACKAGE-0411` (`completed`): committed source에서 signed installer·portable·Ed25519·payload 검증
4. `CP-DOCS-0411` (`completed`): package evidence, canonical freshness, handoff·final report

## Path와 stop condition

- canonical_extension_path: `package.json + desktop Cargo manifests/locks → scripts/package-dioxus.mjs → installer.nsi/portable.nsi → sign-update.mjs`
- package script만 release 산출물을 쓰고 앱 source는 package output을 역으로 읽지 않는다.
- private key 경로와 내용은 기록·출력·commit하지 않는다. embedded public key 일치만 package script가 확인한다.
- mandatory automated check, version identity, helper build, NSIS `/WX`, payload, Ed25519, size/hash 중 하나라도 실패하면 package 완료를 선언하지 않는다.
- review_mode: `SELF_REVIEW`; 독립 reviewer 미참여
- rollback: package 실패 시 public state 변화 없음. source 버전 commit은 local checkpoint로 보존하고 실패 원인을 handoff한다.
- publication: 금지; `release:github`, push, tag, GitHub API를 사용하지 않는다.
