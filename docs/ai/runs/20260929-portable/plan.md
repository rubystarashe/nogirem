# 구현 계획

- DEV-PORT-01 (`IMPLEMENTED`): 포터블 표시 파일과 소유 파일 SHA-256 매니페스트가 포함된 ZIP 생성
- DEV-PORT-02 (`IMPLEMENTED`): 설치형 `update.json`과 분리된 `portable-update.json` 생성·검증
- DEV-PORT-03 (`IMPLEMENTED`): 포터블 모드 감지 및 공유 AppData 설정 유지
- DEV-PORT-04 (`IMPLEMENTED`): 안전한 ZIP 해제, 앱 종료 대기, 소유 파일 교체, 상태 확인, 롤백
- DEV-PORT-05 (`VERIFIED`): Rust·Node 테스트 및 실제 무서명 패키징 검증
- DEV-PORT-06 (`BLOCKED`): 배포용 매니페스트 서명 검증
  - 로컬 Ed25519 개인키가 없어 서명 단계만 실행하지 못했다.

모든 구현 파일의 소유자는 `cursor-92251f36`이며 외부 작업자와 공유 파일 충돌은 없었다.
