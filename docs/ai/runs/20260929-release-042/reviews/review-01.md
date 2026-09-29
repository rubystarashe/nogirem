# 업데이트 수명주기 독립 리뷰

- review_id: `REV-RELEASE-042-01`
- round_id: `ROUND-01/02`
- run_id: `20260929-release-042`
- checkpoint_id: `CP-RELEASE-042-01`
- reviewer: `generalPurpose agent bc9c7542-4947-4127-a76b-237896c235a4`
- review_mode: `INDEPENDENT_REVIEW`
- requested_at: `2026-09-29T13:42:00Z`
- final_at: `2026-09-29T13:47:00Z`
- target: `main@caf15cd3bca07cd106a5688118bc2bd163b07540 + uncommitted release-042 diff`

## 요청과 acknowledgment

- 네트워크 기본 경로 부재, backend 건강 확인, NSIS PowerShell 실행, updater/helper 종료, 프로세스 재탐색과 테스트의 실제 diff·주변 코드를 검토했다.
- reviewer는 파일을 수정하지 않았고 Windows process 수명주기와 하위 호환을 중점 검토했다.

## Round 1 finding

- `ISSUE-UPDATER-001`
- severity: `HIGH`
- must_fix: `true`
- state: `RESOLVED`
- finding: 현재 배포된 0.4.1 updater는 새 installer를 `/S`만으로 실행하므로 새 `/NOGIREMUPDATE` 인자만 신뢰하면 installer가 부모 updater를 종료해 건강 확인·rollback·완료 기록을 잃는다.
- affected_boundary: `update_install.rs → installer.nsi → stop-installed-app.ps1`
- requested_correction: 인자를 모르는 이전 updater도 실제 부모 process와 보호 작업 경로를 검증해 보존하고 실제 회귀를 추가한다.

## DEV response

- 새 updater는 `/NOGIREMUPDATE=<pid>`를 전달하고 installer가 `-UpdaterPid`로 넘기게 했다.
- 이전 updater는 `CallerPid`의 `Win32_Process.ParentProcessId`를 조회하고 `%LOCALAPPDATA%\NogiremUpdater\updates` 아래 정확한 `updater.exe`인 경우 그 PID 하나만 보존한다.
- 직접 설치·제거는 부모가 아닌 stale helper를 계속 종료한다.
- 복사한 Node `updater.exe`가 installer 자식을 시작하는 실제 부모 관계 회귀를 추가해 인자 없는 helper 생존을 확인했다.

## Round 2 rereview

- `ISSUE-UPDATER-001`: `RESOLVED`
- additional_findings: `none`
- validation: targeted Node 39/39, full Node 198/198, PowerShell syntax, `git diff --check`
- verdict: `APPROVED`
- limitations: reviewer는 전체 Rust suite를 재실행하지 않고 DEV의 Rust 결과와 관련 diff를 검토했다.
- stale: `false` for reviewed source; 이후 관련 source 변경 시 재검토 필요

## 영향

- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — 이전 updater와 새 installer 하위 호환성`
- feature_map: `no_change — review는 정본 feature map 변경 없이 구현을 평가함`
- architecture_impact: `updater→installer 부모 process 보존 계약`
- architecture_contract: `no_change — review는 정본 contract 변경 없이 구현을 평가함`
