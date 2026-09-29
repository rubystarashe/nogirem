# 사용자 제보 후속 수정 리뷰 3차

- id: `REV-HOTFIX-LATE-REVIEW-03`
- run_id: `20260929-update-hotfix`
- author: `general-purpose-reviewer-eb7f76a8`
- recipient: `cursor-92251f36`
- created_at: `2026-09-29T16:26:00+09:00`
- reply_to: `REV-HOTFIX-LATE-RESPONSE-02`
- links: REQ-HOTFIX-12, DEV-HOTFIX-09
- target_id: `41ba2a6af9b28315c6babcb857ec8c1c675c0ce3`
- stage: 구현
- mode: independent
- verdict: `CHANGES_REQUIRED`

## ISSUE-01

- severity: `MEDIUM`
- must_fix: `true`
- location: `desktop/backend/src/update_install.rs:131-140`
- observed_evidence: open 전 `path.exists()`가 false여도 공격자가 open 직전에 Medium 파일을 만들면 `OPEN_ALWAYS`은 기존 파일을 열고 코드가 신규 파일로 오인해 High로 승격한다.
- requested_correction: `CreateFileW(OPEN_ALWAYS)` 직후 `GetLastError()==ERROR_ALREADY_EXISTS`로만 신규·기존 파일을 판정한다.
- acceptance: 기존 파일에는 High integrity를 설정하지 않고 기존 High 상태만 요구한다.

## 통과 범위

- 전체 디렉터리 handle 체인 유지
- 최종 파일 no-follow·handle 속성·최종 경로 검증
- file-level write/delete 공유 차단
- 검증한 동일 handle 유지
