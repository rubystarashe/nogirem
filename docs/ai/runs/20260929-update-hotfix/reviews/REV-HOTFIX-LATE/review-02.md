# 사용자 제보 후속 수정 리뷰 2차

- id: `REV-HOTFIX-LATE-REVIEW-02`
- run_id: `20260929-update-hotfix`
- author: `general-purpose-reviewer-eb7f76a8`
- recipient: `cursor-92251f36`
- created_at: `2026-09-29T16:23:00+09:00`
- reply_to: `REV-HOTFIX-LATE-RESPONSE-01`
- links: REQ-HOTFIX-12, DEV-HOTFIX-09
- target_id: `602b8ccd7d7602152eee7f5d08c3c95f5831477f`
- stage: 구현
- mode: independent
- verdict: `CHANGES_REQUIRED`

## ISSUE-01

- severity: `MEDIUM`
- must_fix: `true`
- location: `desktop/backend/src/update_install.rs:128-133`
- observed_evidence: 보호 캐시 검증 후 로그 open까지 디렉터리 handle 체인이 유지되지 않고 최종 파일 open이 reparse를 따라갈 수 있다.
- requested_correction: `lock_safe_directory()` handle 체인을 파일 open까지 유지하고 최종 파일도 `FILE_FLAG_OPEN_REPARSE_POINT`와 handle 속성·최종 경로로 검증한다.
- acceptance: 검증한 동일 파일 handle에 High integrity를 확인한 뒤 그 handle만 append에 사용한다.

## 유지된 개선

- 상승 전 `%APPDATA%` 기록 제거
- panic 원문을 고정 `PANIC` 단계로 제한
- interactive·helper의 상승 성공 뒤 로그 활성화
