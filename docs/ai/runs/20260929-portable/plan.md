# 구현 계획

- DEV-PORT-01 (`IMPLEMENTED`): 내부 앱을 보호된 임시 런타임에 준비하는 단일 포터블 EXE 생성
- DEV-PORT-02 (`IMPLEMENTED`): 설치형 `update.json`과 분리된 `portable-update.json` 생성·검증
- DEV-PORT-03 (`IMPLEMENTED`): 포터블 모드 감지 및 공유 AppData 설정 유지
- DEV-PORT-04 (`IMPLEMENTED`): 앱·launcher 종료 대기, 원본 EXE 원자 교체, 상태 확인, 롤백
- DEV-PORT-05 (`VERIFIED`): Rust·Node 테스트 및 실제 무서명 패키징 검증
- DEV-PORT-06 (`BLOCKED`): 배포용 매니페스트 서명 검증
  - 로컬 Ed25519 개인키가 없어 서명 단계만 실행하지 못했다.

모든 구현 파일의 소유자는 `cursor-92251f36`이며 외부 작업자와 공유 파일 충돌은 없었다.
