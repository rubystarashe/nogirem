# QA 결과

- QA-PORT-01 / REQ-PORT-01 / DEV-PORT-01: `PASS`
  - `npm run package:win:unsigned`
  - 0.4.1 설치 EXE와 7.7MB 단일 포터블 EXE 생성 성공
- QA-PORT-02 / REQ-PORT-01,02 / DEV-PORT-01,02,04: `PASS`
  - 단일 EXE NSIS launcher 컴파일과 포터블 업데이트 자산명 계약 검증 성공
- QA-PORT-03 / REQ-PORT-02~06 / DEV-PORT-02~04: `PASS`
  - `cargo test --locked --manifest-path desktop/backend/Cargo.toml`
- QA-PORT-04 / REQ-PORT-01~06 / DEV-PORT-01~04: `PASS`
  - `cargo check --locked --manifest-path desktop/Cargo.toml`
- QA-PORT-05 / REQ-PORT-01,02,04,05 / DEV-PORT-01~04: `PASS`
  - 포터블·설치기 계약 테스트 7개 통과
- QA-PORT-05A / REQ-PORT-01~06 / DEV-PORT-01~05: `PASS`
  - 최종 전체 Node 테스트 184개와 백엔드 테스트 48개 통과
- QA-PORT-06 / REQ-PORT-02 / DEV-PORT-06: `BLOCKED`
  - 배포용 개인키가 없어 `portable-update.json` 실서명 산출물은 생성하지 못했다.
- QA-PORT-07 / REQ-PORT-02,06: `NOT_RUN`
  - 실제 이전 버전 포터블 앱에서 새 버전으로 갱신하는 종단간 실행은 다음 버전 자산과 서명키가 필요하다.
