# 0.4.7 터보키 제거

- run_id: `20261015-disable-turbo-047`
- checkpoint_id: `CP-DISABLE-TURBO-047`
- owner: `Cursor Agent`
- created_at: `2026-10-15T13:00:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `deb6e254c1bd51ed9f3239de115b000c7955bb31`
- roles: `Cursor Agent = Coordinator + DEV + QA + Feature owner + Architecture owner + Documentation maintainer`; review는 별도 agent에 요청
- feature_impact: `FEAT-NOGIREM-TURBO-KEY — 기능 목록 제거, 신규 실행 차단, 기존 enabled 설정 시작 비활성화`
- feature_map: `updated — lifecycle REMOVED와 0.4.7 동작 기록`
- architecture_impact: `Dioxus UI, Rust backend, Electron 전환 호환 startup·IPC의 저수준 입력 helper 실행 경계`
- architecture_contract: `updated — 제거된 helper 실행 금지와 설정 migration 책임 기록`

## 요청·범위·위험

- 요청: 터보키 구현 코드는 보존하되 기능 목록에서 제거하고 실행되지 않게 막는다. 기존 사용자의 활성화 설정도 앱 시작 시 자동 실행되지 않게 하며 버전을 0.4.7로 기록한다.
- 포함: Dioxus 고급 기능 목록·초기 상태 조회, Rust backend startup·IPC, Electron 전환 호환 startup·IPC, 버전 파일·변경 기록·정본 문서.
- 제외: 기존 helper source·설치 파일 자동 삭제, package·배포·push, unrelated `NOTICE.md`·`data/`·`native/installer-cleanup/`.
- risk: `MEDIUM — 저수준 입력 기능을 폐쇄하는 동작 변경이지만 durable data는 기존 설정의 enabled 한 필드만 false로 migration하고 설치 파일·다른 입력 기능·외부 계약은 보존한다. 교차 저장소·schema·배포 변경은 없다.`
- authorization: source 수정·검사·로컬 commit만 허용. package·배포는 수행하지 않는다.

## 계획·상태

1. `COMPLETED` — 고급 기능 목록과 시작 turbo 상태 조회 제거
2. `COMPLETED` — Rust·Electron startup에서 종료 요청과 `enabled=false` migration 수행
3. `COMPLETED` — 설정·다운로드 실행 요청 거부, helper 구현·제거 경로 보존
4. `COMPLETED` — 0.4.7 버전·변경 기록·feature map·architecture contract 갱신
5. `COMPLETED` — 자동 검사, exact target self-review와 독립 review
6. `COMPLETED` — handoff·최종 기록과 로컬 commit

## 수용 기준·QA

- `TURBO-005`: 고급 기능 목록에 터보키 카드·다운로드·사용 버튼이 없다.
- `TURBO-006`: 기존 `enabled=true` 설정은 Rust와 Electron 시작 경로에서 false로 저장되고 helper를 launch/update하지 않는다.
- `TURBO-007`: Rust와 Electron의 설정·다운로드 요청은 0.4.7 지원 종료 오류를 반환한다.
- `TURBO-008`: `inputs.rs`, `service/main.mjs`, `native/turbo-key`의 구현 source와 설치 제거 코드는 보존한다.
- required checks: turbo Node 회귀, backend tests, backend parity, desktop locked check, 전체 Node tests, `git diff --check`.
- manual installed-upgrade QA: `NOT_RUN — 실제 0.4.6 enabled 사용자 설정과 설치본 환경이 없음`

## 현재 상태

- DEV: `DONE`
- QA: `COMPLETE_PASS`
- review: `APPROVED`, `INDEPENDENT_REVIEW`
- target: `base deb6e254c1bd51ed9f3239de115b000c7955bb31 + uncommitted owned files`
- unrelated_preserved: `NOTICE.md`, `data/`, `native/installer-cleanup/`

## 구현·QA·리뷰

- implemented: Dioxus 고급 기능의 터보키 카드와 초기 상태 요청을 제거했다. Rust·Electron은 설정·다운로드 IPC를 거부하며 제거 IPC와 구현 source는 보존한다.
- migration: 기존 `enabled=true`일 때 키·간격·설치 파일을 보존한 채 설정을 먼저 false로 저장한다. 그 뒤 기존 helper에 종료 control을 기록하며 control 실패는 비활성화 저장을 되돌리지 않는다. 터보키를 사용하지 않은 사용자의 디렉터리·파일은 생성하지 않는다.
- `PASS` — `npm test`: 206/206
- `PASS` — backend: 115 PASS, 3 declared ignore, parity 2/2
- `PASS` — desktop: 8/8
- `PASS` — `cargo check --locked --manifest-path desktop/Cargo.toml`
- `PASS` — `node --check service/main.mjs`
- `PASS` — 변경 파일 IDE lint 0, `git diff --check`
- `NOT_RUN` — 실제 0.4.6 설치본의 enabled 설정으로 0.4.7 앱을 처음 실행하는 수동 upgrade QA
- review_id: `REV-DISABLE-TURBO-047`
- review_mode: `INDEPENDENT_REVIEW`
- reviewer: `generalPurpose agent 2cf29853-9e73-4248-bf94-56847959e022`
- round_1: `CHANGES_REQUESTED — 불필요한 startup write·control 실패 순서, 제거 UI를 요구하는 smoke, migration 실행 검증 부족`
- DEV_response: `기존 enabled 사용자만 설정을 먼저 비활성화하도록 순서를 수정하고, 비사용자 무부작용·옵션 보존 temp directory 검사를 추가했으며 smoke를 미노출 검사로 교체`
- round_2: `APPROVED — 기존 must-fix 3건 해결, unresolved finding 0`
- review_limitations: `실제 0.4.6 설치본 upgrade와 전체 desktop smoke는 reviewer가 실행하지 않음`

## 완료 gate

- feature_gate: `PASS — FEAT-NOGIREM-TURBO-KEY를 REMOVED로 갱신하고 TURBO-005~008과 자동 근거 기록`
- architecture_gate: `PASS — UI·IPC·startup 실행 경계 폐쇄, enabled migration 단일 writer와 no-fallback 정책 기록`
- review_qa_gate: `PASS — 필수 자동 검사와 독립 리뷰 APPROVED, unresolved must-fix 0`
- target_authorization_gate: `PASS — owned 18개 파일만 commit e3a40177b89679fb40942d0a1e04b41d23595b31에 포함하고 사용자 NOTICE.md와 비추적 파일을 보존`
- feature_impact: `FEAT-NOGIREM-TURBO-KEY — REMOVED, 기존 enabled 설정 자동 비활성화`
- feature_map: `updated — revision 2와 0.4.7 검증 evidence 기록`
- architecture_impact: `저수준 입력 helper의 제품 실행 경계 폐쇄와 startup migration`
- architecture_contract: `updated — revision 27`
- final_status: `COMPLETED`
- source_target: `e3a40177b89679fb40942d0a1e04b41d23595b31`
- commit_status: `local commit completed; push·package·deployment NOT_RUN`
