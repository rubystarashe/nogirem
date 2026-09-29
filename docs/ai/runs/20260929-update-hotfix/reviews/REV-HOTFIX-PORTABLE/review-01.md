# 포터블 업데이트 보강 리뷰 1차

- id: `REV-HOTFIX-PORTABLE-REVIEW-01`
- run_id: `20260929-update-hotfix`
- author: `general-purpose-reviewer-686b4860`
- recipient: `cursor-92251f36`
- created_at: `2026-09-29T15:57:00+09:00`
- reply_to: `REV-HOTFIX-PORTABLE-REQUEST-01`
- links: REQ-HOTFIX-08, REQ-HOTFIX-09, REQ-HOTFIX-10, DEV-HOTFIX-07
- target_id: `ebb4d32fbd7fc889671f5febd2eef3e9919110a8`
- stage: 구현
- mode: independent
- verdict: `CHANGES_REQUIRED`

## ISSUE-01

- severity: `MEDIUM`
- must_fix: `true`
- location: `desktop/backend/src/update_install.rs:311-316`, `:147-164`, `:329-334`
- observed_evidence: 동기 롤백 성공 후 `portable-recovery.json`이 남는다. 정리 로직은 이 파일이 있으면 제외하고 복구 탐색은 `installing`만 처리하므로 `rolled-back` 작업이 영구 잔존한다.
- requested_correction: 롤백 교체 성공 후에만 복구 marker를 `portable-complete.json`으로 원자 전환하고 교체 실패 시에는 보존한다.
- acceptance: 해당 상태가 24시간 후 정리 대상이 되는 회귀 테스트를 추가한다.

## 나머지 범위

digest 무효화, share mode 실행 파일 잠금, High mandatory integrity, reparse 검사, NSIS 캐시 사용 중 분기, 복구 후 자동 실행·시작 등록 차단에서는 추가 필수 결함을 찾지 못했다.

## 실제 QA 공백

- UAC 환경 성공 업데이트·강제 실패·전원 중단 복구
- OneDrive/reparse 경로 거부
- digest 불일치 캐시 사용 중 처리
- 정상 업데이트 후 무결성 수준 복원과 재실행
