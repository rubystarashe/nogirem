# 0.4.2 최종 보고

- record_id: `FINAL-RELEASE-042-01`
- run_id: `20260929-release-042`
- checkpoint_id: `CP-RELEASE-042-01`
- owner: `cursor-coordinator-92251f36`
- completed_at: `2026-09-29T14:06:00Z`
- disposition: `COMPLETED`

## 요청과 제공 범위

- 요청: 0.3.16 FastPing 실패 진단, Rust 동일 결함 수정, 건강 확인·PowerShell·updater/helper·제거 재탐색 개선, 실제 회귀와 0.4.2 배포.
- 제공: Electron 호환 및 Rust 네트워크 경로, Rust service/update helper, 설치형·포터블 NSIS, 종료 PowerShell, 실제 process 회귀, 정본 문서, 서명 설치형·포터블과 전환 자산을 배포했다.
- source target: `86ca9261ab763e57477c7655d3f303b256d29bd8`
- remote tag: annotated `v0.4.2`, peeled source가 위 commit과 일치
- release: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.2`

## 구현·진단 결과

- 사용자 진단 시점의 FastPing DWORD는 둘 다 1이어서 미적용 상태가 아니었다. 동시에 `ERR_INTERNET_DISCONNECTED`가 있었고 IPv4 기본 route 부재가 PowerShell terminating error로 노출되는 공통 결함을 수정했다.
- `healthy.json`을 시작 animation 완료에서 backend RPC와 main UI open 성공 시점으로 이동했다.
- 설치·제거 PowerShell을 console 없는 runner로 변경하고 포터블 정상 무프로세스의 `exit 1`을 제거했다.
- 보호 updater cache와 설치 폴더 process를 경로 제한으로 관리하고 종료 대기 중 새 process를 반복 탐색한다.
- 이전 0.4.1 updater가 별도 인자 없이 새 installer를 실행해도 실제 parent PID와 보호 경로를 검증해 보존한다.

## 검증

- automated-test-passing:
  - Node 198/198
  - Rust backend 56 executed with 1 declared live-network ignore
  - Rust desktop compile
  - 실제 임시 process 기반 caller 보존, stale updater 종료, late process 재탐색, 0.4.1 parent updater 호환
  - signed package native contract, NSIS warning-as-error, Ed25519 signing
- manually exercised:
  - startup focused UI smoke `PASS`
  - updater focused UI smoke `PASS`
  - public installer·portable 전체 download와 SHA-256 검증 `PASS`
- failed:
  - full `npm run app:smoke`의 기존 base UI `Advanced structure failed: null`; 관련 focused smoke는 별도 통과
- not_run:
  - 외부 사용자 장치의 실제 앱 내 0.4.1→0.4.2 update
  - 실제 회선 단절·복구 중 사용자 UI 수동 확인
- not_applicable: cross-repository integration은 단일 repository release이므로 적용하지 않는다.

## 리뷰와 gate

- review_mode: `INDEPENDENT_REVIEW`
- verdict: `APPROVED`
- mandatory finding: `ISSUE-UPDATER-001 RESOLVED`
- approval freshness: reviewed source 이후 제품 source 변경 없음
- feature_gate: `PASS` for defined automated release scope
- architecture_gate: `PASS`
- review_qa_gate: `PASS` with disclosed pre-existing full-smoke gap
- target_authorization_gate: `PASS`
- integration_gate: `NOT_APPLICABLE — 단일 repository`

## 배포와 자산

- installer: `303a9008f2bdd4a42c93cab7e9594e47f953617ab6d2d6a31ff4859f0d92ee04`
- portable: `fd956491fe940c9c02c8faa5782c03b0afbe395f24c19ea7f46dad0e15c57ad7`
- public manifest signatures: `PASS`
- GitHub draft/prerelease: `false/false`
- release latest: `true`
- commit: 로컬 commit 완료
- push: release annotated tag만 push, `main` branch는 push하지 않음
- deployment: GitHub v0.4.2 자산 공개 완료
- activation: 사용자 장치 설치·업데이트는 수행하지 않음

## 영향과 남은 위험

- feature_impact: `FEAT-NOGIREM-NETWORK-FASTPING — route 부재 처리`, `FEAT-NOGIREM-UPDATE-LIFECYCLE — 준비 신호·process 종료·하위 호환`
- feature_map: `updated — docs/ai/wiki/feature-map.md`
- architecture_impact: `NIC GUID writer, updater single writer, installer/helper trust와 process lifetime`
- architecture_contract: `updated — docs/ai/wiki/agent-friendly-architecture.md`
- known risks: 기존 advanced structure smoke harness gap, 실제 외부 장치 update·인터넷 복구 수동 확인 부재.
- exceptions: 없음

## CP-RELEASE-042-02 대치 배포 최종 보고

- completed_at: `2026-09-29T18:16:00Z`
- disposition: `COMPLETED — 공개 자산 대치`, 실제 외부 관리자 토큰 종단간은 `NOT_RUN`
- request: `신뢰할 수 없는 업데이트 작업 파일입니다` 오판 수정과 v0.4.2 대치 재배포.
- source_target: `3cb35d86bf8967a86303b9440754445d9d338443`
- delivered: PowerShell SDDL 문자열 비교를 Win32 label ACL 조회로 교체하고 ACL·ACE·SID 경계, authority, NO_WRITE_UP와 High/System RID를 검증한다. `icacls /C`가 실제 설정 실패를 성공처럼 숨기던 경로도 제거했다.
- automated-test-passing: Node 198/198, Rust backend 57 executed with 2 declared ignores, desktop cargo check, release package checks.
- manually exercised: GitHub 공개 설치형·포터블·두 manifest 전체 다운로드, SHA-256과 Ed25519 payload 연결 검증.
- review: `INDEPENDENT_REVIEW APPROVED`; `ISSUE-INTEGRITY-001/002 RESOLVED`; approval fresh.
- feature_gate: `PASS for source and automated scenarios`; 실제 0.4.1→대치 0.4.2 외부 장치 update는 `NOT_RUN`.
- architecture_gate: `PASS`; native mandatory-label contract revision 2.
- target_authorization_gate: `PASS`; assets만 대치했고 tag history와 main branch를 변경하지 않았다.
- release: [v0.4.2](https://github.com/rubystarashe/nogirem/releases/tag/v0.4.2)
- installer: `8ed390dbcc809fbaf3fbe84a591b1bda6ebb7974f7079f503030d39d2caa95c0`
- portable: `128d4a3d0a381cbf2ad4bee183fb4d4a64dc0ae80635a9cfed2e4f37277d2b0c`
- limitation: 기존 0.4.1 helper는 대치 payload 실행 전에 실패하므로 영향 사용자는 0.4.2를 한 번 수동 설치해야 한다. Cursor sandbox의 `icacls` 오류 1299로 실제 High label path query는 실행하지 못했다.
- commit: `3cb35d86bf8967a86303b9440754445d9d338443`; push `none`; tag rewrite `none`; 사용자 장치 activation `none`.
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — 작업 파일 신뢰 오판 수정`
- feature_map: `updated — docs/ai/wiki/feature-map.md revision 2`
- architecture_impact: `mandatory integrity ACL parsing and setting failure boundary`
- architecture_contract: `updated — docs/ai/wiki/agent-friendly-architecture.md revision 2`
