# 0.4.2 조정 상태

- record_id: `COORD-RELEASE-042-01`
- run_id: `20260929-release-042`
- checkpoint_id: `CP-RELEASE-042-01`
- owner: `cursor-coordinator-92251f36`
- updated_at: `2026-09-29T14:06:00Z`
- target: `nogirem@86ca9261ab763e57477c7655d3f303b256d29bd8`, remote tag `v0.4.2`

## 역할과 소유

- Coordinator + DEV + QA + Feature owner + Architecture owner + Documentation maintainer: `cursor-agent-92251f36`
- Reviewer: `generalPurpose agent bc9c7542-4947-4127-a76b-237896c235a4`
- mutable owners: DEV는 앱·스크립트·테스트, Documentation maintainer는 `docs/ai/wiki`, Coordinator는 이 실행의 조정·인계·최종 보고를 소유한다.
- review_mode: `INDEPENDENT_REVIEW`

## 상태

- active_checkpoint: `CP-RELEASE-042-01 completed`
- DEV: `DONE`
- QA: `COMPLETE_PASS` for mandatory release scope
- review: `APPROVED`, `ISSUE-UPDATER-001 RESOLVED`
- release: `PUBLISHED`, public GitHub Latest Release
- dirty baseline: 작업 시작 시 기존 staged·unstaged·untracked 변경 없음
- concurrent work: 발견되지 않음

## 결정

- 진단 ZIP은 패스트핑 미적용을 입증하지 않는다. 보고 시점 값은 적용 완료였으며 인터넷 단절 중 기본 경로 쿼리의 예외 노출을 결함으로 처리한다.
- 새 updater는 `/NOGIREMUPDATE=<pid>`를 전달하며 이전 updater는 검증된 실제 부모 관계로 식별한다. 정확한 부모 helper 하나만 보존하고 직접 설치·제거는 stale helper를 제거한다.
- 프로세스 종료는 이미지 이름이 아니라 설치 폴더 또는 보호된 updater 작업 경로의 정규화된 실행 파일 경로로 제한한다.
- 공개 배포는 자동·독립 검증과 리뷰가 모두 통과한 같은 소스 target에서만 진행한다.

## 영향

- feature_impact: `FEAT-NOGIREM-NETWORK-FASTPING — 기본 경로 부재 처리`, `FEAT-NOGIREM-UPDATE-LIFECYCLE — 건강 확인과 설치·제거 프로세스 제어`
- feature_map: `updated — docs/ai/wiki/feature-map.md`
- architecture_impact: `updater·installer·helper 신뢰 경계와 single update writer, NIC GUID writer 경계`
- architecture_contract: `updated — docs/ai/wiki/agent-friendly-architecture.md`

## CP-RELEASE-042-02

- updated_at: `2026-09-29T18:16:00Z`
- active_checkpoint: `completed`
- DEV: `DONE`
- QA: `COMPLETE_PASS for executable automated scope`
- review: `APPROVED`, `ISSUE-INTEGRITY-001/002 RESOLVED`
- source: `3cb35d86bf8967a86303b9440754445d9d338443`
- release: `PUBLISHED_REPLACEMENT`, GitHub v0.4.2 assets
- compatibility: 0.4.2 대치본은 이후 updater 검증을 수정한다. 이미 오류에 진입한 0.4.1은 0.4.2 수동 설치가 필요하다.
- target decision: 기존 remote tag는 재작성하지 않고 assets만 명시 승인에 따라 대치했다. main branch는 push하지 않았다.
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE`
- feature_map: `updated — revision 2`
- architecture_impact: `native mandatory-label trust boundary`
- architecture_contract: `updated — revision 2`

## CP-RELEASE-042-03

- updated_at: `2026-09-30T10:14:00Z`
- active_checkpoint: `completed`
- DEV: `DONE`
- QA: `COMPLETE_PASS`
- review: `APPROVED`, `ISSUE-ASSET-001~004 RESOLVED`
- source: `ce0bd8cda6265373e68cf2c23e3d1f2f87045e5d`
- release: `PUBLISHED`, GitHub v0.4.2 named aliases and Electron feed
- decision: Rust canonical manifest names stay `nogirem-dioxus-*` for deployed-client compatibility. Short names are user-facing aliases and labels distinguish both roles.
- old Electron assets: removed only after long-name asset, blockmap, latest feed and full public download verification.
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE`
- feature_map: `updated — revision 3`
- architecture_impact: `dual asset naming and legacy Electron feed`
- architecture_contract: `updated — revision 3`
