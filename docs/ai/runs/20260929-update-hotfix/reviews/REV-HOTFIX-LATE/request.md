# 사용자 제보 후속 수정 리뷰 요청

- id: `REV-HOTFIX-LATE-REQUEST-01`
- run_id: `20260929-update-hotfix`
- author: `cursor-92251f36`
- recipient: `general-purpose-reviewer-eb7f76a8`
- created_at: `2026-09-29T16:18:00+09:00`
- reply_to: 늦게 완료된 포터블 보안 변경 검토
- links: REQ-HOTFIX-04, REQ-HOTFIX-09, REQ-HOTFIX-11, REQ-HOTFIX-12, DEV-HOTFIX-08, DEV-HOTFIX-09
- target_id: `5782ac004023aaa202d570380c476fcae1cb3c65`
- base_commit: `f6178df57dbb17383b90a2ab18125a21754a9bec`
- target_commit: `5782ac004023aaa202d570380c476fcae1cb3c65`
- stage: 구현
- mode: independent

## 요청 작업

- 일반 업데이트 작업에 `result.json`이 helper 시작 전에 생성돼 `trusted_job` 계약을 만족하는지 검토한다.
- 포터블 대상의 상위 디렉터리 handle 체인이 reparse·rename 경쟁을 교체 전에 차단하는지 검토한다.
- NSIS가 현재 설치기·제거기 PID를 종료 스크립트에 전달하고 스크립트가 호출자를 제외하는지 검토한다.
- 예약 작업 action의 `Execute`가 null인 경우 제거가 중단되지 않는지 검토한다.
- GUI WebView 생성 전 bootstrap 로그가 경로·인자 등 민감정보 없이 실패 단계를 남기는지 검토한다.

## 검증 근거

- 전체 Node 테스트 193개 통과
- Rust backend 테스트 56개 통과, 공개 실다운로드 검사 1개 기본 제외
- 실제 NTFS junction 거부·상위 디렉터리 rename 잠금 테스트 통과
- 실제 제거 모드 PowerShell에서 호출자 제거기 생존과 정상 종료 확인
- Rust desktop 관련 7개 통과, 기존 Markdown fixture 1개 실패

## 미검증 영역

- 제보 사용자 환경의 WebView2·startup 로그
- 실제 UAC 설치·제거 UI 종단간 실행
