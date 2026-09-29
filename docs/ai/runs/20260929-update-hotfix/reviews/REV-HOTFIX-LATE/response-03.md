# 사용자 제보 후속 수정 리뷰 3차 응답

- id: `REV-HOTFIX-LATE-RESPONSE-03`
- run_id: `20260929-update-hotfix`
- author: `cursor-92251f36`
- recipient: `general-purpose-reviewer-eb7f76a8`
- created_at: `2026-09-29T16:28:00+09:00`
- reply_to: `REV-HOTFIX-LATE-REVIEW-03`
- links: REQ-HOTFIX-12, DEV-HOTFIX-09
- target_id: `0b5683eee69396f41b56c367c03ae425ab37ce27`
- base_commit: `41ba2a6af9b28315c6babcb857ec8c1c675c0ce3`
- target_commit: `0b5683eee69396f41b56c367c03ae425ab37ce27`
- stage: 구현
- mode: independent
- state: `READY_FOR_REREVIEW`

## ISSUE-01

- disposition: 수정
- changes:
  - `CreateFileW(OPEN_ALWAYS)` 직전에 last error를 0으로 초기화한다.
  - 호출 성공 직후 다른 Win32 호출 전에 `GetLastError()==ERROR_ALREADY_EXISTS`를 저장한다.
  - 기존 파일에는 integrity를 설정하지 않고 기존 High integrity 검증만 수행한다.
  - 신규 파일에만 High integrity를 설정한다.
- evidence:
  - Rust backend 56개 통과
  - Rust desktop `cargo check` 통과
  - 생성 판정·no-follow·제거 계약 테스트 6개 통과

## 재검토 요청

open 직전 파일 주입이 기존 파일 승격으로 이어지지 않는지 최종 재검토한다.
