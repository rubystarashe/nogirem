# 리뷰 현황

- review_id: `REV-HOTFIX-C6866`
- mode: `independent`
- reviewer: Cursor general-purpose agent `c6866e71-4fc0-4f66-9592-d6dbe8404c2c`
- scope: Electron 0.3.18 전환, Rust 0.4.1 업데이트·종료, 설치·제거 프로세스 경계
- rounds:
  - 1: `CHANGES_REQUIRED` — 오류 UI, UAC, 전역 프로세스 종료, 종료 경쟁, 최소 버전
  - 2: `CHANGES_REQUIRED` — handoff 이후 동시 재시도
  - 3: `CHANGES_REQUIRED` — apply helper 준비 확인
  - 4: `CHANGES_REQUIRED` — helper 오류 분기 회수
  - 5: `APPROVED`
- final_status: `APPROVED`
- final_target: 최종 미커밋 코드 스냅샷, 승인 후 동일 코드로 실서명 산출물 재생성
- remaining_review_limit: 실제 Windows 설치 전환과 공개 자산 교체는 코드 리뷰 범위 밖

- review_id: `REV-HOTFIX-3474`
- mode: `independent`
- reviewer: Cursor general-purpose agent `3474f0da-d6b0-455d-a213-dff16e746c69`
- scope: reqwest blocking 다운로드, Rust 오류 UI·숨김·재시도, 회귀 검사
- rounds:
  - 1: `CHANGES_REQUIRED` — 닫을 수 없는 오류 패널과 기본 검사에서 제외된 실다운로드 회귀
  - 2: `CHANGES_REQUIRED` — 숨김 버튼 추가 후 설치 버튼 스모크 선택자 충돌
  - 3: `APPROVED`
- final_status: `APPROVED`

- review_id: `REV-HOTFIX-E3CC`
- mode: `independent`
- reviewer: Cursor general-purpose agent `e3cc111c-01b9-4695-8105-de0dff343f92`
- scope: Electron 0.3.18 화면 숨김, helper 로그와 timeout 재시도 경쟁
- rounds:
  - 1: `CHANGES_REQUIRED` — 이전 helper가 새 시도 상태를 오염시키는 경쟁
  - 2: `CHANGES_REQUIRED` — timeout 뒤 기존 attempt 무효화 누락
  - 3: `APPROVED`
  - 4: `CHANGES_REQUIRED` — 0.3.18 클라이언트의 0.3.19 라우팅 직접 검사와 최종 경로 기록 누락
  - 5: `APPROVED`
- final_status: `APPROVED`
