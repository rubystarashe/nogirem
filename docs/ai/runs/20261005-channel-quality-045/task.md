# 0.4.5 채널 네트워크 품질 개선

- run_id: `20261005-channel-quality-045`
- checkpoint_id: `CP-CHANNEL-QUALITY-045`
- owner: `Cursor Agent`
- created_at: `2026-10-04T22:49:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `a68c85f1da24a19e19305dc1f4b058ce72c0cc29`
- roles: `Cursor Agent = Coordinator + DEV + QA + Feature owner + Architecture owner + Documentation maintainer`, `independent reviewer = package 전 지정`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — 단순 성공 평균을 안정성·실패율을 포함한 쉬운 품질 등급과 실제 게임 연결 RTT로 보완`
- feature_map: `updated — 0.4.5 측정·표시·검증 범위를 구현 후 반영`
- architecture_impact: `channel ping scheduler·rolling outcomes·Windows TCP EStats 수집 활성화·조회 경계`
- architecture_contract: `updated — 유휴 무부하 원칙, 실제 게임 instance·5-tuple 검증과 connection-scoped TCP 통계 수집 경계를 추가`

## 요청·범위·위험

- 요청: 기존 TCP 연결 시간만으로 채널을 단정하지 않고 안정성까지 반영한다. 사용자는 `안정적`, `보통`, `불안정`, `연결 불가`처럼 쉬운 용어만 보게 한다.
- 버전: 제품 버전을 `0.4.5`로 올리고 `VERSION_HISTORY.md`에 변경 기록을 추가한다.
- 성능 제약: 측정 횟수를 늘리지 않고 최근 기록만 더 보관한다. 채널 측정은 60초 동안 분산하고 실제 게임 TCP 통계는 유휴 polling 없이 overlay 요청 때만 확인한다.
- 제외: 게임 프로토콜 패킷 전송·주입, 패킷 캡처 드라이버, 임의 endpoint, ICMP 기반 품질 단정, 배포.
- risk: `HIGH — Windows TCP 연결 소유자·게임 PID 신뢰 경계와 저수준 네트워크 통계, 사용자 추천 의미를 변경`

## 상태·수용 기준

- DEV: `IN_PROGRESS`
- QA: `PLANNED`
- review: `NOT_REVIEWED`
- release: `NOT_AUTHORIZED`

- 기존 1분당 채널 측정량을 늘리지 않고 짧은 동시 burst를 제거한다.
- 최근 20회 성공·실패를 함께 보관하고 중앙값·변동·실패율로 품질을 계산한다.
- 1~2ms 수준의 불확실한 차이는 공동 추천으로 처리한다.
- overlay에는 기술 용어 대신 `안정적`, `보통`, `불안정`, `연결 불가`, `확인 중`을 표시한다.
- 실제 게임 연결 RTT는 검증된 Client.exe PID와 CSV endpoint가 일치하고 게임이 foreground일 때만 읽는다.
- 기능 비활성·게임 미실행 유휴 상태에서 새 polling·network operation이 없어야 한다.

## 범위 추가 — 게임 중 상시 표시

- added_at: `2026-10-04T23:01:00Z`
- requester: `user`
- 요청: 기능명을 `채널별 핑`에서 `실시간 핑 확인`으로 바꾸고, 실제 연결된 채널 IP:포트를 식별해 게임 화면 상단 중앙에 초소형 상태를 상시 표시한다.
- 동작: 게임이 foreground이고 CSV endpoint와 일치하는 TCP 연결이 있을 때 `채널 · 실제 RTT · 쉬운 상태`를 표시한다. Windows 키를 누르면 기존 전체 채널표로 확장하고 다음 입력 시 초소형 표시로 복귀한다.
- 성능 결정: 게임이 foreground인 동안에만 1초 주기로 TCP EStats를 갱신한다. 게임이 아니거나 기능이 꺼지면 실제 TCP 조회와 창 표시를 중지한다.
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — 실시간 핑 확인으로 명칭 변경, foreground 게임의 실제 접속 채널 상시 표시 추가`
- feature_map: `updated — 상시 compact와 Windows 키 expanded 흐름을 반영`
- architecture_impact: `overlay 상태가 hidden/compact/expanded로 확장되고 foreground·실제 TCP 연결이 표시 권한을 결정`
- architecture_contract: `updated — CSV endpoint 및 검증된 게임 process instance만 connection-scoped TCP EStats 대상으로 허용`

## QA·리뷰·최종 상태

- updated_at: `2026-10-04T23:20:00Z`
- DEV: `READY_FOR_QA`
- QA: `BLOCKED — 자동 범위는 통과했으나 실제 마비노기 실행 환경이 없어 actual RTT·채널 전환·DPI·물리 입력을 실행하지 못함`
- review: `APPROVED — REV-CHANNEL-QUALITY-045 round 2, ISSUE-045-001~003 RESOLVED`
- release: `NOT_AUTHORIZED`
- final_disposition: `PARTIALLY_COMPLETED`

### 실행 결과

- `cargo test --locked -p nogirem-backend`: `PASS — 89 passed, 2 declared ignored, integration 2/2`
- `npm test`: `PASS — 204/204`
- `cargo check --locked -p nogirem-desktop`: `PASS`
- `node --test test/overlay-interface.test.mjs`: `PASS — 3/3`
- `git diff --check`: `PASS`
- IDE diagnostics: `PASS — edited source 0 errors`
- 실제 게임 현재 채널·EStats RTT·채널 전환: `NOT_RUN — 실행 중인 마비노기와 안전한 fixture 없음`
- compact 236×30 위치·혼합 DPI·expanded 물리 입력 복귀: `NOT_RUN — 실제 게임 창 없음`

### 구현 결과

- 제품 버전과 lock metadata를 `0.4.5`로 갱신하고 `VERSION_HISTORY.md`에 변경 기록을 추가했다.
- 38개 동시 burst를 제거하고 60초에 걸쳐 endpoint를 하나씩 측정하며 최근 20회 성공·실패로 중앙값·MAD·최대 변동·실패율을 계산한다.
- 사용자 표시는 `안정적`, `보통`, `불안정`, `연결 불가`, `확인 중`으로 제한하고 종합 점수 2ms 이내를 공동 추천한다.
- 전경 게임 process handle을 조회 완료까지 유지하고 CSV IPv4:port와 일치하는 established 5-tuple만 선택한다. 복수 후보는 이전 identity 또는 단일 RTT 후보가 아니면 fail-closed한다.
- 동일 connection의 EStats RTT·변동과 retransmit·timeout 증가를 반영하고 조회·collection 활성화 실패를 `activeConnectionError`로 노출한다.
- 게임 전경 연결 중 236×30 compact를 상시 표시하고 Windows 키로 584×400 expanded를 연 뒤 다음 down 입력에 compact로 복귀한다.

### Gate

- feature_gate: `BLOCKED — source·자동 계약은 갱신됐으나 실제 게임 behavior verification NOT_RUN`
- architecture_gate: `PASS — process instance·5-tuple·EStats collection·single writer 경계와 독립 review 승인`
- review_gate: `PASS — independent APPROVED, unresolved must-fix 0`
- qa_gate: `BLOCKED — actual game·EStats·DPI·physical input mandatory manual scenarios NOT_RUN`
- target_gate: `PASS — implementation commit a1c8699c4abaf97c86b9d712f1f60e613442e669, push·package·release 미수행`
- commit_status: `local commit complete — a1c8699c4abaf97c86b9d712f1f60e613442e669`
