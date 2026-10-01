# 하드 프리즈 진단 로깅

- run_id: `20261001-freeze-diagnostics`
- checkpoint_id: `CP-LOGGING`
- owner: `Cursor Agent`
- created_at: `2026-10-01T12:59:00Z`
- target: `nogirem/main`, base `14e620b8f791da0e317b4c128c7400abfb5c9982`, 초기 working tree clean
- roles: `Cursor Agent = Coordinator + DEV + QA + Reviewer + Documentation maintainer`
- review_mode: `SELF_REVIEW — 독립 검토자 없음`
- feature_impact: `none: 최적화 동작은 바꾸지 않고 기존 하드 프리즈 원인 판별용 진단 증거만 추가`
- feature_map: `no_change — 기존 제품 기능·수명주기·수용 기준은 변경되지 않음`
- architecture_impact: `none: affinity writer와 Windows event 조회 경계를 유지하며 관측 정보만 추가`
- architecture_contract: `no_change — 기존 경계가 그대로 정확함`

## 요청

진단 `a1ffcb53-5d92-4375-b503-f65fbc9568bf`에서 확인된 게임 종료·재실행 시 하드 프리즈를 판별할 수 있도록 로깅을 추가한다.

## 범위와 위험

- affinity helper의 게임 감지, 적용, 복원, 비정상 종료 복구 전환을 bounded JSONL 로그로 남긴다.
- 진단 ZIP에 최근 WHEA, GPU driver, Display, LiveKernelEvent를 추가한다.
- 사용자 파일 내용, 계정, 입력 키 기록, 전체 프로세스 경로는 수집하지 않는다.
- 낮은 위험: 기존 affinity·DXVK·입력 동작과 durable 설정 writer를 변경하지 않고 전환 시점의 관측만 추가한다. 진단 생성 시간과 로그 I/O가 제한적으로 증가한다.

## 계획과 상태

1. `COMPLETED` — 기존 로깅·진단 경계 확인
2. `COMPLETED` — affinity 전환 JSONL과 Windows hardware event 수집 구현
3. `COMPLETED` — Rust·Node 검사, lint, SELF_REVIEW
4. `COMPLETED` — handoff 갱신과 로컬 checkpoint commit

## 검증 계획

- affinity event 로그의 JSONL 형식, 회전, 복원 단계 기록 unit test
- backend 전체 test와 compile
- Node 진단 계약 test
- 변경 diff SELF_REVIEW 및 민감정보 수집 여부 확인

## 현재 disposition

- DEV: `DONE`
- QA: `COMPLETE_PASS`
- review: `APPROVED`
- status: `COMPLETED`

## 구현 결과

- `desktop/backend/src/affinity.rs`
  - `affinity/events.log`와 `events.log.previous`에 최대 1MiB씩 JSONL을 유지한다.
  - helper 생성, 게임 시작 감지, affinity 적용 완료, 게임 종료 감지, 복원 시작·완료·실패, 시작 복구 사유를 기록한다.
  - 대상·변경·복원·누락·identity 변경·실패 수, mask, 경과 시간, ISLC·Process Lasso 충돌을 남긴다.
  - 로그는 전환마다 flush와 `sync_data`를 실행해 강제 재부팅 직전 기록의 생존 가능성을 높인다.
- `desktop/backend/src/diagnostics.rs`
  - 기존 종료 이벤트와 별도로 WHEA, Display, `nvlddmkm`, `amdwddmg`, DxgKrnl 이벤트를 수집한다.
  - Windows Error Reporting의 `LiveKernelEvent`와 `BlueScreen`을 수집한다.
  - 존재하지 않거나 조회할 수 없는 provider는 개별적으로 격리해 전체 진단 생성을 막지 않는다.
- `.handoff/cursor.md`
  - 신규 관측 범위와 아직 남은 실제 하드 프리즈 후 검증 공백을 기록했다.

## QA와 증거

- `PASS` — `cargo test --locked --manifest-path desktop/backend/Cargo.toml`: 58 passed, 2 environment-dependent ignored, integration/parity passed.
- `PASS` — `cargo test --locked --manifest-path desktop/backend/Cargo.toml diagnostics::tests`: 3 passed.
- `PASS` — `cargo check --locked --manifest-path desktop/Cargo.toml`.
- `PASS` — `node --test test/application-reliability.test.mjs`: 21 passed.
- `PASS` — 변경 PowerShell을 현재 Windows에서 실행: shutdown 7, hardware/display 5, live-kernel 0을 유효 JSON으로 반환.
- `PASS` — IDE lint: 변경 Rust 파일 오류 없음.
- `PASS` — `git diff --check`.
- `FAIL 후 수정` — 최초 선택 test 명령은 Cargo filter 인자를 두 개 전달해 실행되지 않았다. 전체 backend test로 교체했다.
- `FAIL 후 수정` — 최초 hardware provider 조회는 일부 미등록 provider가 stderr를 출력했다. provider별 `try/catch`로 격리한 뒤 실실행을 통과했다.
- `NOT_RUN` — 실제 하드 프리즈 후 신규 0.4.2 진단 생성. 사용자 환경에서 다음 재현 후 수행해야 한다.

## SELF_REVIEW

- reviewer: `Cursor Agent`
- reviewed_at: `2026-10-01T13:10:00Z`
- target: base `14e620b8f791da0e317b4c128c7400abfb5c9982`, implementation commit `1eb7fe44e4100d6640d232ba03ce8dd9926e6fc9`
- inspected: affinity 전환 전후 기록 순서, 기존 apply/restore 의미 보존, bounded file rotation, 강제 flush, PowerShell provider 실패 격리, 진단 redaction, 민감정보 수집 범위, 테스트와 미검증 주장
- findings: `must_fix 0`
- verdict: `APPROVED`
- limitations: 동일 actor의 SELF_REVIEW이며 독립 reviewer는 참여하지 않았다. 실제 하드 프리즈 후 로그 생존은 미검증이다.

## 완료 게이트

- feature gate: `PASS — 제품 동작 변경 없음, 정본 feature map 변경 불필요`
- architecture gate: `PASS — 기존 affinity writer·진단 조회 경계 유지, 신규 위반 없음`
- review and QA gate: `PASS WITH LIMITATION — 자동 검사 통과, 독립 리뷰와 실제 하드 프리즈 후 검증 없음`
- target and authorization gate: `PASS — 명시 파일만 포함한 implementation commit 1eb7fe44e4100d6640d232ba03ce8dd9926e6fc9 확인`
- commit/push/deployment: `local commit 1eb7fe44e4100d6640d232ba03ce8dd9926e6fc9, push NOT_RUN, deployment NOT_RUN`
