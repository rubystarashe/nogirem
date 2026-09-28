# 리뷰 현황

- REVIEW-PORT-INDEPENDENT-01
  - stage: implementation
  - mode: independent
  - reviewer: `07734892-c52d-45f3-87fe-dba8bf8ccaf1`
  - target: 현재 미커밋 포터블 배포 변경
  - round 1: `CHANGES_REQUIRED`
    - reparse 경로, 수정 가능한 이전 매니페스트, 사용자 파일 충돌, 중단 복구, 시작 경로 복구, 서명 전 ZIP 검증 문제
  - round 2: `CHANGES_REQUIRED`
    - 인증되지 않은 캐시 복구 자료와 실행 중 본체 롤백 문제
  - round 3: `CHANGES_REQUIRED`
    - 사용자 쓰기 가능한 캐시의 helper·스테이징 교체 경쟁
  - round 4: `CHANGES_REQUIRED`
    - 캐시 루트 junction 우회 문제
  - round 5: `APPROVED`
    - 전용 캐시의 단계별 reparse 검사, High integrity 보호, canonical 경계 확인 후 릴리스 차단 결함 없음
  - 한계: 실서명 버전 간 업데이트와 강제 중단 롤백 종단간 QA는 별도 수행 필요

- REVIEW-PORT-INDEPENDENT-02
  - stage: single-exe correction
  - mode: independent
  - reviewer: `63321661-0afb-46f0-adfc-fe9b6c6a5f77`
  - round 1: `CHANGES_REQUIRED`
    - 사용자 쓰기 가능 임시 추출, 중단 복구 누락, 실패 범위, 원본 경로 검증 문제
  - round 2: `CHANGES_REQUIRED`
    - 임시 경로 보호 전 경쟁과 완료 기록 실패의 허위 롤백 문제
  - round 3: `APPROVED`
    - 관리자 전용 런타임, launcher 대기, 원본 PID 검증, 원자 완료 마커와 백업 복구 확인
  - 한계: 실서명 패키지 강제 중단 복구 종단간 QA는 별도 수행 필요
