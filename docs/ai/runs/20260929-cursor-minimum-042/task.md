# 0.4.2 커서 최소 크기 확장

- record_id: `RUN-CURSOR-042`
- run_id: `20260929-cursor-minimum-042`
- checkpoint_id: `CP-CURSOR-042-01`
- owner: `cursor-coordinator-92251f36`
- roles: `Coordinator + DEV + QA + Documentation maintainer`
- created_at: `2026-09-29T13:20:00Z`
- updated_at: `2026-09-29T13:25:00Z`
- base_target: `main@e165063d5cba5c60e4453faeecbac6fe872000ac`
- status: `COMPLETED`

## 요청과 해석

- 원문: `커서 최소 크기 옵션을 더 늘려. 0.4.2 기능으로 해`
- 해석: 기존 75%~800% 범위의 하한에 25%와 50%를 추가하고 앱 버전을 0.4.2로 올린다.
- 제외: 800% 상한, 25% 증분, 게임 포그라운드 제한, 원래 크기 복원 방식은 변경하지 않는다.

## 계획과 소유

1. `native/input-guard-helper/main.cpp`, Rust 설정 검증, 두 Rust UI 목록, 레거시 Electron 정규화의 하한을 25%로 통일한다.
2. 자동 테스트가 25% 허용, 24%·801% 거부, 휠 하한 25%, UI 선택지 25%·50%를 검증하게 한다.
3. 패키지와 Rust crate 버전을 0.4.2로 올리고 lockfile과 변경 기록을 동기화한다.
4. helper 빌드, 관련 Node/Rust 테스트, Rust 컴파일, 자체 리뷰를 수행한다.
5. `.handoff/cursor.md`와 이 기록을 완료 상태로 갱신한 뒤 소유 파일만 커밋한다.

## 위험과 영향

- risk: `LOW`
- rationale: 기존 커서 크기 기능의 허용 하한과 선택 목록만 확장한다. durable data 형식, writer, IPC 필드, 권한, 상한, 복원 계약은 바뀌지 않는다. 25%는 기존 Windows 적용 경계 1~256 안에서 원래 크기를 비례 축소한다.
- feature_impact: `FEAT-INPUT-CURSOR-SCALE — 25%·50% 하한 선택과 휠 축소 범위 추가`
- feature_map: `no_change — 별도 정본 feature map이 없는 현재 저장소에서 기존 기능의 범위만 확장하며 이 실행 기록과 버전 기록을 정본 근거로 남긴다`
- architecture_impact: `none: 기존 UI → service → input helper → Windows SystemParametersInfo 경계와 단일 writer를 유지한다`
- architecture_contract: `no_change — 새 의존성·writer·저장소·호출 경계를 추가하지 않는다`

## 검증 계획

- `node --test test/input-guard.test.mjs`
- `cargo test --locked --manifest-path desktop/backend/Cargo.toml inputs::tests`
- `cargo check --locked --manifest-path desktop/Cargo.toml`
- `npm run native:input-guard`
- review_mode: `SELF_REVIEW`
- independent_reviewer: `none`

## 실행 결과

- helper 빌드: `PASS`
  - `npm run native:input-guard`
  - output: `native/input-guard-helper/bin/input-guard-helper.exe`
  - SHA-256: `203E0D26EA1516AC75448384027CE5F0EA33D7642C1C10F4E1E1C33FAA00B47A`
- 커서 계약 테스트: `PASS`, 3/3
  - `node --test test/input-guard.test.mjs`
- 전체 Node 회귀: `PASS`, 194/194
  - `npm test`
- Rust backend: `PASS`, 56 executed and 1 live-network test ignored by declaration
  - `cargo test --locked --manifest-path desktop/backend/Cargo.toml`
- Rust desktop compile: `PASS`
  - `cargo check --locked --manifest-path desktop/Cargo.toml`
- IDE lint: `PASS`, 변경 소스 진단 없음
- whitespace: `PASS`
  - `git diff --check`
- manual_product_qa: `NOT_RUN`
  - 실제 마비노기 포그라운드에서 25% 커서 표시를 눈으로 확인하는 실행은 수행하지 않았다.

## 자체 검토

- review_id: `REV-CURSOR-042-SELF-01`
- review_mode: `SELF_REVIEW`
- verdict: `APPROVED`
- inspected: helper 입력 검증·휠 하한·Windows 적용 clamp, Rust 저장·runtime·patch 검증, 두 UI 목록, 레거시 정규화, 버전·lockfile, 테스트와 재빌드 binary
- findings: `none`
- disclosure: 구현자와 검토자가 동일하며 독립 검토자는 참여하지 않았다.

## 완료 판정

- DEV: `DONE`
- QA: `COMPLETE_PASS` for automated scope
- feature_gate: `PASS — 25%·50% 선택, 저장, helper 전달, 휠 하한을 자동 검증했다`
- architecture_gate: `PASS — 기존 경계와 single writer를 유지했다`
- review_qa_gate: `PASS — low-risk 변경의 SELF_REVIEW와 자동 검증 완료`
- target_authorization_gate: `PASS — main 기준 소유 파일만 변경했고 push·배포·활성화하지 않았다`
- remaining: 실제 게임 포그라운드에서 25% 가시성 수동 확인
