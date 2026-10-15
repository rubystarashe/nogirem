# 0.4.6 자동 업데이트 설치 hotfix

- record_id: `RUN-20261015-UPDATER-HOTFIX-046`
- run_id: `20261015-updater-hotfix-046`
- checkpoint_id: `CP-UPDATER-HOTFIX-046`
- owner: `Cursor Agent`
- roles: `Coordinator=DEV=QA=Reviewer=Feature owner=Architecture owner=Documentation maintainer=Cursor Agent`
- review_mode: `INDEPENDENT_REVIEW`
- created_at: `2026-10-15T11:50:00Z`
- updated_at: `2026-10-15T12:18:00Z`
- source_base: `36e24da646d91b6bcde35ac3aea06c16c4274d63`
- source_target: `6c8b48d9713654d79a8467e8df371ca7a81ef59f`
- dev_state: `READY_FOR_QA`
- qa_state: `BLOCKED — 공개 또는 격리된 signed 이전 버전→hotfix 실행 target이 없음`
- review_verdict: `APPROVED`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — 0.4.x 설치형·포터블의 작업 파일 신뢰 판정과 설치기 handoff`
- feature_map: `updated — 작업 파일별 명시적 High Integrity seal과 검증된 updater의 단일 종료 책임을 반영`
- architecture_impact: `update transaction의 보호 파일 seal, updater와 NSIS 종료 책임 경계`
- architecture_contract: `updated — 상속 레이블 비의존과 검증된 updater handoff 뒤 중복 프로세스 탐색 금지`

## Request

- original_request: `0.4.6 자동 업데이트 적용 시 "신뢰할 수 없는 업데이트 작업 파일입니다", 설치기의 "실행 중인 앱을 정상 종료한 뒤 다시 설치", helper의 "설치 프로그램 실패: exit code: 1"로 중단되는 제보를 확인하고 수정`
- objective: 두 실패 단계를 분리 재현하고, 서명·digest·reparse 방어를 유지한 채 정상 updater handoff가 설치까지 진행되도록 수정한다.
- scope: Rust update job 준비·승인 파일 seal, 신뢰 오류 식별성, NSIS 종료 script의 검증된 updater 경로, 회귀 테스트, signed package 후보 생성과 검증.
- exclusions: 사용자 PC 원격 조작, GitHub Release 자산 대치, tag·push·배포, 실제 게임 실행.
- authorization: 로컬 진단·수정·검증·자동 local commit은 사용자 요청과 정책 범위. 공개 배포는 미승인.
- acceptance:
  - 모든 helper 입력과 commit marker가 명시적으로 High Integrity로 설정되고 즉시 재검증된다.
  - 신뢰 판정 실패는 정확한 파일명을 포함한다.
  - 정확한 보호 cache의 `updater.exe` PID만 중복 설치 프로세스 탐색을 생략할 수 있다.
  - 직접 설치·제거와 인자 없는 0.4.1 updater 호환 종료 경로는 유지된다.
  - Rust, installer 회귀, package 검증이 통과한다.
- risk: `HIGH — 관리자 권한 update transaction, 실행 파일 writer, rollback과 공개 배포 payload에 영향을 줌`
- stop_condition: 수정·필수 자동 검사·signed package 검증·독립 리뷰·정본 문서·handoff·local commit까지 완료하고 배포하지 않는다.

## Findings

### FB-UPDATER-046-001

- classification: `DEFECT`
- severity: `BLOCKER`
- must_fix: `true`
- reported_target: `공개 0.4.6으로 업데이트하는 배포된 0.4.x`
- observed: `신뢰할 수 없는 업데이트 작업 파일입니다`, helper exit `255`.
- cause: 보호 디렉터리의 상속 레이블에 의존해 개별 job 입력과 설치 승인 파일의 High Integrity를 생성 직후 확정하지 않았고, 오류가 실패 파일을 식별하지 않았다.
- disposition: `FIXED — 모든 신뢰 입력과 후속 result·포터블 복구 파일을 생성·교체 때마다 seal·재검증하고 오류에 파일명을 포함`

### FB-UPDATER-046-002

- classification: `DEFECT`
- severity: `BLOCKER`
- must_fix: `true`
- reported_target: `공개 0.4.6 설치형 자동 업데이트`
- observed: 설치기가 `실행 중인 앱을 정상 종료한 뒤 다시 설치해 주세요`, helper가 `설치 프로그램 실패: exit code: 1`을 표시.
- cause: updater가 기록한 app·backend 종료와 worker flush를 완료한 뒤에도 NSIS 종료 script가 설치 폴더를 다시 탐색해 종료 중 보조 프로세스를 일반 앱으로 오판할 수 있었다.
- disposition: `FIXED — 설치기 실제 부모 PID와 보호 cache의 정확한 updater.exe가 모두 일치할 때만 중복 설치 폴더 탐색을 생략`

## Plan and ownership

1. `COMPLETED` — 공개 manifest·payload 다운로드와 두 제보 단계 분리.
2. `COMPLETED` — update job·후속 state·포터블 복구·commit seal, 실제 부모에 결속된 updater handoff 수정과 회귀 테스트.
3. `COMPLETED` — 독립 리뷰 반영 후 signed 0.4.6 package 재생성·package contract 검증.
4. `COMPLETED` — source target, 정본 freshness·handoff와 local commit.

- mutable_file_owner: `Cursor Agent — desktop/backend/src/update_install.rs, desktop/backend/src/service.rs, scripts/stop-installed-app.ps1, test/installer.test.mjs, affected canonical docs`
- out_of_scope_dirty_preserved: `data/, native/installer-cleanup/`
- canonical_extension_path: `Updater::prepared → update_install::prepare/seal → protected updater.exe → NSIS /NOGIREMUPDATE=<verified PID> → health check/rollback`
- durable_writer: `Rust updater만 job/result/commit과 update transaction을 쓰며 NSIS만 설치 파일을 쓴다`
- rollback: 설치형 기존 파일·설정·registry backup과 건강 확인 실패 rollback을 유지한다.

## Validation

- `PASS` — 공개 `update.json` redirect·Ed25519 manifest·0.4.6 payload 실다운로드.
- `PASS` — 실제 보호 download payload에 대한 native mandatory-label 조회.
- `PASS` — `cargo check --locked --offline`.
- `PASS` — backend `105 passed`, parity `2 passed`, `3 declared ignored`.
- `PASS` — installer 회귀 `10/10`, 실제 updater parent 결속·위조 PID 거부·직접 설치·제거·0.4.1 parent 호환 포함.
- `PASS` — 전체 Node `206/206`.
- `PASS` — signed package `release/dioxus-0.4.6-2026-10-15T12-17-45-499Z`, packaged native contract, NSIS `/WX`, 10MB 제한, Ed25519 manifest 자체 검증.
- installer: `9,576,758 bytes`, SHA-256 `3327260676e2b754314956ef2ba2f625452f2070f2430e6868bb1560afd56920`
- portable: `7,795,362 bytes`, SHA-256 `a03315ef4cf647d00fa4075b9503284761804506ec4c7f87adf58ec56450a2b1`
- `NOT_RUN` — 공개 GitHub 자산 대치와 외부 사용자 PC 종단간 재시도.

## Review

- review_mode: `INDEPENDENT_REVIEW`
- reviewer: `Cursor generalPurpose agent 86fe4c6f-46b8-446a-8fa5-41be299c0d26`
- round_1: `CHANGES_REQUESTED — 전달 PID가 설치기 실제 부모에 결속되지 않음; 후속 result·포터블 복구 파일 seal 지속성 누락`
- dev_response: `CallerPid ParentProcessId와 보호 cache updater.exe를 함께 검증하고 위조 PID 거부 회귀 추가. trusted_state와 포터블 recovery/backup/helper 생성별 seal 추가.`
- round_2: `APPROVED — 두 must-fix RESOLVED, 신규 must-fix 0`
- limitations: reviewer는 실제 관리자 update transaction과 signed package 실행을 수행하지 않았다.
- verdict: `APPROVED`

## Current disposition

- status: `PARTIALLY_COMPLETED — source·signed candidate 완료, 실제 이전 버전 적용과 배포 미실행`
- commit: `6c8b48d9713654d79a8467e8df371ca7a81ef59f`
- push: `NOT_RUN`
- tag: `NOT_RUN`
- deployment: `NOT_RUN`
