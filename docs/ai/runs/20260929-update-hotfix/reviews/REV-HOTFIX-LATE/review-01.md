# 사용자 제보 후속 수정 리뷰 1차

- id: `REV-HOTFIX-LATE-REVIEW-01`
- run_id: `20260929-update-hotfix`
- author: `general-purpose-reviewer-eb7f76a8`
- recipient: `cursor-92251f36`
- created_at: `2026-09-29T16:19:00+09:00`
- reply_to: `REV-HOTFIX-LATE-REQUEST-01`
- links: REQ-HOTFIX-04, REQ-HOTFIX-09, REQ-HOTFIX-11, REQ-HOTFIX-12
- target_id: `5782ac004023aaa202d570380c476fcae1cb3c65`
- stage: 구현
- mode: independent
- verdict: `CHANGES_REQUIRED`

## ISSUE-01

- severity: `MEDIUM`
- must_fix: `true`
- location: `desktop/src/main.rs:21-28`, `:54-55`
- observed_evidence: 상승된 프로세스가 사용자 제어 `%APPDATA%` 아래 로그 경로를 따라 쓰며 panic 원문에 경로·사용자 데이터가 포함될 수 있다.
- requested_correction: 전체 로그 경로의 reparse를 거부하고 handle 기반 보호 경로에서만 쓰거나 상승 프로세스의 사용자 쓰기 경로 기록을 제거한다. panic은 허용 목록의 고정 단계명만 기록한다.
- acceptance: 보호된 로그 경로와 고정 panic 문자열을 확인하고 raw panic 정보가 남지 않는다.

## 승인된 범위

- 업데이트 준비 작업은 helper 전에 모든 신뢰 입력을 생성한다.
- 상위 디렉터리 handle 체인이 junction과 rename 경쟁을 차단한다.
- NSIS 설치·제거 CallerPid 전달, 호출자 제외, null 예약 작업 action 처리가 적절하다.
