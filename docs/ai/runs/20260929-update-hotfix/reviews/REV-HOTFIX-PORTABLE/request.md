# 포터블 업데이트 보강 리뷰 요청

- id: `REV-HOTFIX-PORTABLE-REQUEST-01`
- run_id: `20260929-update-hotfix`
- author: `cursor-92251f36`
- recipient: `independent-general-purpose-reviewer`
- created_at: `2026-09-29T15:56:00+09:00`
- reply_to: 없음
- links: REQ-HOTFIX-08, REQ-HOTFIX-09, REQ-HOTFIX-10, DEV-HOTFIX-07
- target_id: `ebb4d32fbd7fc889671f5febd2eef3e9919110a8`
- base_commit: `e56b3b67819b9e73c047d973f5b0701bb2d53204`
- target_commit: `ebb4d32fbd7fc889671f5febd2eef3e9919110a8`
- stage: 구현
- mode: independent

## 요청 작업

- 동일 버전 포터블 교체본의 런타임 캐시 무효화가 실제 NSIS 흐름에서 동작하는지 검토한다.
- `--recover-portable-exe`와 실패 롤백이 임의 파일의 관리자 실행·시작 등록으로 이어지지 않는지 검토한다.
- 사용자 쓰기 가능 경로의 staging, reparse, 이동 전후 해시, 열린 handle이 교체·실행 경쟁을 차단하는지 검토한다.
- 완료·롤백·중단 캐시 정리가 활성 복구 작업을 삭제하지 않는지 검토한다.

## 검증 근거

- 전체 Node 테스트 192개 통과
- Rust backend 테스트 52개 통과, 공개 실다운로드 검사 1개 기본 제외
- 실서명 NSIS 설치형·포터블 패키징과 서명 매니페스트 크기·SHA-256 검증 통과
- desktop 테스트 8개 중 변경과 무관한 기존 Markdown fixture 1개 실패

## 미검증 영역

- 실제 UAC 환경에서 성공 업데이트·강제 실패·전원 중단 복구 종단간 실행
- OneDrive 등 reparse 경로에 둔 포터블의 의도된 거부 안내
