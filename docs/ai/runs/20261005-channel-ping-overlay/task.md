# 채널별 핑 오버레이

- run_id: `20261005-channel-ping-overlay`
- checkpoint_id: `CP-CHANNEL-PING-OVERLAY`
- owner: `Cursor Agent`
- created_at: `2026-10-04T19:35:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `ac3abe73a4b32785e5319e84178dd457310a66b6`
- roles: `Cursor Agent = Coordinator + DEV + QA + Feature owner + Architecture owner + Documentation maintainer`, `independent reviewer = generalPurpose agent 8e36f609-a04b-447b-b9f4-e23e6caa50ab`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — 채널 endpoint 동기화, 주기적 지연시간 측정, 게임 전경 Win 키 오버레이`
- feature_map: `updated — 신규 기능 identity와 시나리오·증거·미검증 범위를 추가`
- architecture_impact: `channel.csv 정본·캐시 writer, TCP 측정 worker, foreground input adapter, 클릭 통과 보조 창`
- architecture_contract: `updated — 기능 module·durable writer·저수준 input hook·보조 창 경계를 추가`

## 요청·권한·위험

- 원 요청: 앱 시작 시 GitHub의 `channel.csv`를 최신화하고 1분마다 채널별 평균 핑을 측정한다. 마비노기가 전경일 때 Windows 키를 누르면 게임 중앙에 클릭 불가·항상 위·반투명 인터페이스를 표시하고, 이후 마우스나 키보드의 어떤 down 입력에도 숨긴다. 고급 기능에서 켜고 끌 수 있어야 한다.
- 포함: Rust backend 기능 module, 고정 GitHub raw URL 동기화, 검증된 사용자 캐시, TCP 포트 지연시간 이동 평균, 실패 중 1시간 원격 재검사, Win32 전경·입력 adapter, 보조 WebView 오버레이, 고급 기능 설정, 패키징 입력, 자동 테스트와 정본 문서.
- 제외: ICMP 관리자 권한 변경, 임의 URL 입력, Windows 키 차단, 게임 프로세스 주입, 배포·push.
- risk: `HIGH — 전역 입력 상태를 관찰하고 외부 endpoint에 주기적으로 연결하며 topmost 보조 창과 지속 캐시를 추가함`
- low_risk: `not_applicable — behavioral·network·privacy·durable data·Windows UI 경계가 함께 변경됨`

## 계획·상태

- DEV: `READY_FOR_QA`
- QA: `COMPLETE_PASS — 자동 범위`; `실제 게임 수동 범위 NOT_RUN`
- review: `APPROVED — independent round 5`

1. `COMPLETED` — 기존 IPC·보조 창·게임 식별·패키징 경계와 CSV 형식 확인
2. `COMPLETED` — strict CSV 검증, startup 원격 동기화, 1분 측정과 최근 5회 성공값 평균 구현
3. `COMPLETED` — 마비노기 전경 Windows 키 표시, trigger 입력 release 처리, 모든 후속 down 숨김 구현
4. `COMPLETED` — 클릭 통과·비활성 topmost 반투명 오버레이와 고급 기능 설정 연결
5. `COMPLETED` — 자동 검사·독립 review·정본 문서와 source checkpoint commit 완료

## 수용 기준·시나리오

- `PING-001`: 유효한 원격 CSV는 앱 시작 때 검증 후 사용자 캐시에 원자 저장하고 즉시 endpoint 정본으로 사용한다.
- `PING-002`: 원격 실패 또는 잘못된 CSV는 마지막 유효 캐시, 그다음 패키지 CSV 순으로 복구하며 잘못된 원격 값으로 기존 정본을 덮지 않는다.
- `PING-003`: endpoint가 있는 각 채널은 1분마다 TCP 연결 지연을 측정하고 최근 성공 5회의 평균을 메모리에 유지한다. 빈 endpoint는 `정보 없음`이며 실패 갱신을 유발하지 않는다.
- `PING-004`: 하나 이상의 실제 endpoint 측정이 실패하면 원격 CSV 재검사를 최대 1시간에 한 번 수행한다.
- `PING-005`: 기능이 켜져 있고 마비노기 전경 창에서 좌·우 Windows 키 down이 시작되면 게임 client 영역 중앙에 오버레이가 비활성·click-through·topmost 상태로 나타난다.
- `PING-006`: 표시 trigger 당시 눌린 키는 release까지 무시하되, 그 외 키보드·마우스 down 또는 trigger 키의 다음 down은 오버레이를 즉시 숨긴다.
- `PING-007`: 기능을 끄면 오버레이를 숨기고 입력 감지는 표시 동작을 수행하지 않는다.
- `PING-008`: 패키지에 fallback `channel.csv`, 오버레이 HTML과 preload가 포함된다.

## 데이터·보안·운영

- durable writer: `channel_ping.rs`만 `%APPDATA%\마비노기 렘 부스터\channel-ping\channel.csv`와 `setting.json`을 쓴다.
- authoritative source: 고정 HTTPS GitHub raw URL의 검증된 CSV. 캐시와 패키지 CSV는 네트워크 실패 fallback이다.
- readers: channel ping worker와 오버레이 상태 IPC만 endpoint 정본을 읽는다.
- sensitive data: 사용자 계정·게임 데이터·로컬 주소를 수집하거나 전송하지 않는다. 고정 공개 endpoint에 TCP 연결하고 고정 공개 URL만 조회한다.
- compatibility: Windows 전용 desktop 기능이며 Electron 경로는 변경하지 않는다.
- logs_metrics_alerts: 시작 동기화·측정 오류는 기존 bounded startup log에 오류만 기록하며 IP 외 사용자 데이터는 기록하지 않는다.

## QA·review 기록

- `PASS` — channel ping unit 8/8, 기본 병렬 설정 10회 반복
- `PASS` — backend 81개 중 79 PASS·2 declared ignore, integration 2/2
- `PASS` — Node 203/203
- `PASS` — `cargo check --manifest-path desktop/Cargo.toml --locked`
- `PASS` — 실제 Windows에서 `WH_KEYBOARD_LL`·`WH_MOUSE_LL` 설치·정리 수명주기 unit test
- `NOT_RUN` — 실제 마비노기 전경에서 Windows 키 표시, 모든 물리 입력 즉시 숨김, 혼합 DPI 위치, 실제 endpoint 측정, 원격 GitHub 갱신
- `NOT_RUN` — 설치형·포터블 package 실행. package script 입력 계약만 자동 검사함
- review_mode: `INDEPENDENT_REVIEW`
- reviewer: `generalPurpose agent 8e36f609-a04b-447b-b9f4-e23e6caa50ab`
- round 1: `CHANGES_REQUESTED` — opacity, IPC authorization, mixed DPI, mapped IPv6, queue overflow, hook timeout·disconnect, 반대 Win 키, disable 경쟁, async error 갱신과 테스트 finding
- round 2~4: `CHANGES_REQUESTED` — overflow stale queue, 최초 WebView open block, quiet barrier, 테스트 전역 상태 간섭 finding
- round 5: `APPROVED` — `ISSUE-CP-001~015` resolved, unresolved source must-fix `0`, new finding `0`
- review_target: `source commit 000481dbdc438729d13b54829d4653ab51824c78, base ac3abe73a4b32785e5319e84178dd457310a66b6`
- review_limitations: 실제 게임·혼합 DPI·물리 입력과 package는 reviewer도 실행하지 않음

## 후속 피드백 2026-10-05

- feedback_id: `CP-FEEDBACK-016`
- classification: `DEFECT`, severity: `HIGH`, must_fix: `true`, status: `READY_FOR_VERIFICATION`
- reported behavior: 고급 기능에서 채널별 핑을 켜면 보조 창에 `Unknown auxiliary document`가 표시됨.
- cause: backend 보조 창 등록과 package 입력에는 `channel-ping-overlay.html`이 있었지만 Dioxus native document 허용 목록에서 누락됨.
- disposition: `FIXED — native 허용 목록에 문서를 추가하고 회귀 검사를 보강함`
- related UI feedback: 부스터 정지 화면에서 `터보키 제거하기` 버튼의 흰 배경이 문구에 밀착되어 보이는 문제를 좌우 5px, 최소 너비 72px로 조정함.
- validation: `PASS — node --test test/overlay-interface.test.mjs (3/3)`, `PASS — cargo check --locked --manifest-path desktop/Cargo.toml`, `PASS — 변경 파일 IDE lint 0`
- manual verification: `NOT_RUN — 수정 빌드 재실행 후 채널 핑 활성화 및 정지 화면 버튼 시각 확인 필요`
- review_mode: `SELF_REVIEW — native 허용 경계, 회귀 검사, CSS 선택자 범위를 작성자가 재검토했으며 독립 reviewer는 참여하지 않음`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — 활성화 시 보조 HTML이 실제 오버레이로 열리도록 결함 수정`, `FEAT-NOGIREM-TURBO-KEY — 정지 화면 제거 버튼 가독성 조정`
- feature_map: `no_change — 기존 정본 동작 정의는 정확하며 구현 누락과 시각 결함만 수정`
- architecture_impact: `native auxiliary document allowlist의 기존 보조 창 경계 보완`
- architecture_contract: `no_change — 기존 허용 목록을 통한 보조 문서 제한 정책을 그대로 따름`

### 입력·표시 후속

- feedback_id: `CP-FEEDBACK-017`
- classification: `DEFECT`, severity: `HIGH`, must_fix: `true`, status: `READY_FOR_VERIFICATION`
- reported behavior: 오버레이가 표시된 뒤 키보드 또는 마우스 down에도 닫히지 않음.
- cause: 감시 루프가 비동기 보조 창 상태를 매 회 다시 읽어 표시 직후 상태 갱신 경쟁이 발생할 수 있었음.
- disposition: `FIXED — 감시 루프가 성공한 show/hide 결정과 함께 overlay_visible을 직접 추적하고 입력 판단에 사용함`
- feedback_id: `CP-FEEDBACK-018`
- classification: `REQUIREMENT_CLARIFICATION`, severity: `MEDIUM`, must_fix: `true`, status: `READY_FOR_VERIFICATION`
- requested behavior: 잘린 외곽 반투명 여백·그림자를 제거하고 채널을 `1~15`, `16~29`, `30~38` 세로 3열로 표시함.
- disposition: `FIXED — body 외곽 여백과 box-shadow를 제거하고 세 범위별 compact list column으로 재배치함`
- validation: `PASS — channel_ping 관련 10/10`, `PASS — overlay-interface 3/3`, `PASS — desktop locked check`, `PASS — 변경 파일 IDE lint 0`
- manual verification: `NOT_RUN — 수정 빌드에서 물리 입력 숨김, 외곽 마감, 3열 배치 재확인 필요`
- review_mode: `INDEPENDENT_REVIEW`
- reviewer: `generalPurpose agent 35612e97-29bf-40a5-8cec-e6827360900f`
- review_rounds: `CHANGES_REQUESTED 2회 — 창 파괴·재생성 시 로컬 표시 상태와 실제 ID 경쟁`, `APPROVED — 실제 표시 ID 반환·검증, tracked ID와 visible 동시 갱신 및 interleaving 회귀 검사 확인`
- review_target: `base eb70e614a3cd90cc82986034a3dd00507b56e65a, uncommitted diff SHA-256 8cec567c3c15f773e97e8a51f77f5d553b239ddacee912e27c341809a7180092`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — 입력 down 종료 신뢰성과 채널 목록 배치·외곽 시각 마감 변경`, `FEAT-NOGIREM-TURBO-KEY — 제거 버튼 여백 축소`
- feature_map: `no_change — 기존 종료 동작과 채널 표시 기능 정의는 정확하며 결함·표현만 수정`
- architecture_impact: `overlay visibility authority를 비동기 window state에서 input monitor loop의 로컬 상태로 이동`
- architecture_contract: `no_change — 기존 service-owned 보조 창·입력 adapter 경계 안의 상태 추적 수정`

### 실제 QA 재현과 native 즉시 숨김 보강

- feedback_id: `CP-FEEDBACK-019`
- classification: `DEFECT`, severity: `HIGH`, must_fix: `true`, status: `READY_FOR_VERIFICATION`
- reported behavior: 상태 추적 수정 빌드에서도 키보드·마우스 down 후 오버레이가 실제 화면에서 사라지지 않음.
- disposition: `FIXED — native open 응답의 HWND를 window ID와 함께 검증·등록하고, 다음 low-level down callback에서 ShowWindowAsync(SW_HIDE)를 즉시 enqueue한 뒤 기존 상태 이벤트를 처리함`
- safety: 등록·hook hide·파괴는 `window registry state → input registration` 순서로 잠그고, hook은 hide enqueue 반환까지 등록 잠금을 유지한다. 일반 hide/destroy는 raw HWND를 사용하지 않고 native window ID 명령만 사용한다.
- feedback_id: `CP-FEEDBACK-020`
- classification: `REQUIREMENT_CLARIFICATION`, severity: `MEDIUM`, must_fix: `true`, status: `READY_FOR_VERIFICATION`
- requested behavior: 채널명과 핑 간격 축소, 우상단 시각 제거와 로고 watermark, `n초 전에 측정했습니다` 실시간 표시, 5~10ms 노랑·10~15ms 주홍·15ms 이상 빨강, 최저 핑 채널 단일 highlight.
- disposition: `FIXED — 세 열을 각각 170px로 축소하고 각 행 내부는 채널 왼쪽·핑 오른쪽 정렬을 유지함. 콘텐츠 합계에 맞춰 overlay 창도 720px에서 584px로 축소함. 1초 age 갱신, 경계별 색상 및 첫 최저값 강조도 적용함`
- validation: `PASS — channel ping 관련 11/11`, `PASS — overlay-interface 3/3`, `PASS — desktop locked check`, `PASS — JavaScript syntax`, `PASS — 변경 파일 IDE lint 0`
- manual verification: `NOT_RUN — 최신 수정 빌드에서 실제 물리 입력 즉시 숨김과 최종 시각 표현 확인 필요`
- review_mode: `INDEPENDENT_REVIEW`
- reviewer: `generalPurpose agent 7a370aca-f40b-449a-87c1-80340497cfe8`
- review_rounds: `CHANGES_REQUESTED 2회 — raw HWND 등록·파괴 및 검증-사용 경합`, `APPROVED — ISSUE-CP-019 RESOLVED, 새 must-fix 0`
- review_target: `base b95f053804a38e56d1cf34dff21fa6f041d0b4e5, uncommitted diff SHA-256 550b4f0e1174289cd898dad394ecb729ef319320bdf4a2c46b51492bed99f9c9`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — 물리 입력 종료 경로와 지연시간 상태·강조 표현 변경`
- feature_map: `no_change — 기존 종료·측정 기능 정의는 정확하며 구현 신뢰성과 표시만 수정`
- architecture_impact: `native HWND identity 전달, low-level hook hide enqueue, window registry와 input registration 수명 잠금`
- architecture_contract: `no_change — 기존 native window adapter와 input monitor 경계 안에서 lifetime을 결합`

## 최종 상태

- disposition: `PARTIALLY_COMPLETED — source·자동 QA·독립 review 완료, 실제 게임·package QA 미실행`
- commit: `000481dbdc438729d13b54829d4653ab51824c78`
- push: `NOT_AUTHORIZED`
- deployment: `NOT_AUTHORIZED`
