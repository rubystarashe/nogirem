# QA — CPU topology and priority

- record_id: `QA-CPU-TOPOLOGY-PRIORITY-001`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-TOPOLOGY-PRIORITY-001`
- owner: `cursor-agent-92251f36`
- qa_role: `DEV-as-QA`
- qa_state: `COMPLETE_PASS`
- source_target: `EVID-CPU-TOPOLOGY-SOURCE-001; patch SHA-256 f0f3b3d3c3eb1a869ea39f19cc59b569c2cff913865fd98604b128dfe0c30c18`
- execution_target: `Windows 10.0.26200 x64; Rust test/debug target; Node v22.20.0`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — topology allocation, affinity and priority recovery`
- feature_map: `no_change — QA intake only; canonical update follows verified implementation`
- architecture_impact: `topology parser/allocation and process mutation/restore boundaries`
- architecture_contract: `no_change — QA intake only; canonical update follows verified implementation`

## 필수 시나리오

- `SCN-CPU-TOPO-001`: hybrid SMT fixture에서 요청 P-core 수와 disjoint background mask.
- `SCN-CPU-TOPO-002`: AMD single/multiple L3와 SMT on/off locality.
- `SCN-CPU-TOPO-003`: 2C/4T 및 4 physical core fail-safe.
- `SCN-CPU-TOPO-004`: malformed API buffer와 multi-group fail-safe.
- `SCN-CPU-AFF-001`: game binding 기본 false와 opt-in 적용.
- `SCN-CPU-PRI-001`: background priority 기본 false 및 role exclusion.
- `SCN-CPU-REC-001`: normal restore, PID identity mismatch, external mutation preservation and restart recovery.
- `SCN-CPU-IPC-001`: persisted settings, live worker control, service contract and UI status.
- `SCN-CPU-LIVE-001`: Windows child process priority roundtrip.
- `SCN-CPU-GAME-001`: 실제 게임 frame 성능 비교. `NOT_RUN` 예정이며 자동 검증 범위 밖.

## 실행 기록

- `SCN-CPU-TOPO-001`~`004`: `PASS` — `cargo test --locked --manifest-path desktop/backend/Cargo.toml`; hybrid, L3, SMT, 4-core, malformed, multi-group와 live Windows topology 포함.
- `SCN-CPU-AFF-001`: `PASS` — default false settings test, typed worker options, service/UI contract, SELF_REVIEW.
- `SCN-CPU-PRI-001`: `PASS` — role target unit test와 child process `BELOW_NORMAL_PRIORITY_CLASS` roundtrip.
- `SCN-CPU-REC-001`: `PASS` — same-handle implementation/source review, external-value restore guard unit test, durable ledger startup path review.
- `SCN-CPU-IPC-001`: `PASS` — Node `npm test` 213/213, desktop 8/8 and locked desktop check.
- `SCN-CPU-LIVE-001`: `PASS` — `process::tests::child_priority_roundtrip_uses_one_verified_handle`.
- `SCN-CPU-GAME-001`: `NOT_RUN` — 실제 마비노기와 packaged helper 강제 종료 환경이 없어 자동 검사와 분리.

## 명령과 결과

- `cargo test --locked --manifest-path desktop/backend/Cargo.toml`: `PASS`, library 124 passed·3 ignored, integration parity 2/2, doc tests 0.
- `cargo test --locked --manifest-path desktop/Cargo.toml`: `PASS`, 8/8.
- `cargo check --locked --manifest-path desktop/Cargo.toml`: `PASS`.
- `nvm current; npm test`: `PASS`, Node v22.20.0, 213/213.
- `git diff --check`: `PASS`; line-ending conversion notices only.
- IDE diagnostics on changed Rust/UI files: `PASS`, 0 errors.
- `cargo clippy --locked --manifest-path desktop/Cargo.toml --all-targets -- -D warnings`: `FAIL` due repository baseline warnings under Rust 1.98 in unrelated pre-existing code. The one introduced `too_many_arguments` finding was fixed with typed `Options`; strict command remains nonzero because baseline warnings were not broadened or suppressed.

## Coverage assessment

Mandatory source and automated scenarios for this checkpoint passed. Actual game frame/latency behavior remains explicitly `NOT_RUN` and no frame-improvement claim is made. Strict Clippy is a disclosed repository-baseline gap rather than a new violation in the changed implementation.

## Follow-up CP-CPU-MODES-002

- qa_state: `COMPLETE_PASS`
- source_target: `base 17dee9063db8c9926d70a1f9eab5a4955b7c47d8 + follow-up owned-file target`
- execution_target: `Windows 10.0.26200 x64, AMD Ryzen 7 9800X3D 8C/16T, Rust test/debug, Node v22.20.0`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — 게임 집중·균형·안전 mode`
- feature_map: `updated — revision 3`
- architecture_impact: `mode allocation and settings/worker/service/UI contract`
- architecture_contract: `updated — revision 36`

### 시나리오

- `SCN-CPU-MODE-001`: `PASS` — 8P fixture에서 게임 집중 6P, 균형 4P와 background mask를 확인했다.
- `SCN-CPU-MODE-002`: `PASS` — 2P+4E fixture에서 게임 집중 2P와 background 4E+공유 1P를 확인했다.
- `SCN-CPU-MODE-003`: `PASS` — 4P 비하이브리드가 3P game+1P background로 동작하고 물리 코어 수만으로 비활성화되지 않는다.
- `SCN-CPU-MODE-004`: `PASS` — 신규 설정은 focus, legacy bind true는 balanced, false는 safe로 정규화한다.
- `SCN-CPU-MODE-005`: `PASS` — 실행 중 mode control이 game affinity 여부, 자동 core count와 priority 독립 설정을 함께 재구성한다.
- `SCN-CPU-MODE-006`: `PASS` — desktop smoke가 고급 기능 화면의 mode control을 찾아 `균형`을 선택하고 service state가 `balanced`, 4P로 바뀐 것을 확인했다.
- `SCN-CPU-MODE-GAME-001`: `NOT_RUN` — 실제 마비노기 frame time·입력·background 체감 비교는 별도 실게임 QA가 필요하다.

### 현재 실행 결과

- `cargo test --locked --manifest-path desktop/backend/Cargo.toml`: `PASS`, library 126 passed·3 ignored, parity 2/2.
- `cargo check --locked --manifest-path desktop/Cargo.toml`: `PASS`.
- `cargo test --locked --manifest-path desktop/Cargo.toml`: `PASS`, 8/8.
- `nvm use 22.20.0; npm test`: `PASS`, 213/213.
- `npm run app:dev -- --smoke-test --smoke-report=... --capture=...`: `PASS`, report `passed:true`, CPU mode settings roundtrip과 고급 기능 구조 확인.
- `cargo fmt --manifest-path desktop/backend/Cargo.toml -- --check`: `FAIL`, 변경 밖의 기존 compact-format source까지 포함한 repository formatting baseline. 자동 format으로 대규모 무관 diff를 만들지 않았으며 변경 diff는 `git diff --check`를 통과했다.

## Final follow-up CP-CPU-MAIN-PRESETS-004

- qa_state: `COMPLETE_PASS`
- supersedes: `CP-CPU-MODES-002` scenario conclusions where preset composition differs
- source_target: `base 17dee9063db8c9926d70a1f9eab5a4955b7c47d8 + final preset owned-file target`
- execution_target: `Windows 10.0.26200 x64, AMD Ryzen 7 9800X3D 8C/16T, Rust test/debug, Node v22.20.0`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — main focus/balanced/safe presets and advanced custom count`
- feature_map: `updated — revision 5`
- architecture_impact: `preset defaults, settings writer, worker live control and main UI placement`
- architecture_contract: `updated — revision 38`

### 시나리오

- `SCN-CPU-PRESET-001`: `PASS` — 8P fixture에서 focus auto 6P, balanced auto 4P와 manual 6P를 확인했다.
- `SCN-CPU-PRESET-002`: `PASS` — safe는 balanced count를 사용하면서 game mask를 적용하지 않는 manager 경계를 확인했다.
- `SCN-CPU-PRESET-003`: `PASS` — 2P+4E focus에서 game 2P와 background 4E+공유 1P를 확인했다.
- `SCN-CPU-PRESET-004`: `PASS` — SchedulingClass 차이가 같은 EfficiencyClass의 P-core를 제거하지 않고 선택 순서에만 반영된다.
- `SCN-CPU-PRESET-005`: `PASS` — service mode 선택은 custom count를 초기화하고 advanced count 선택은 mode를 보존한다.
- `SCN-CPU-PRESET-006`: `PASS` — desktop smoke가 Vulkan control 아래 세 preset의 DOM 위치·label, focus 6P, balanced/safe 4P와 advanced manual 5P mode 보존을 확인했다.
- `SCN-CPU-PRESET-GAME-001`: `NOT_RUN` — 실제 마비노기 frame time·입력·background 체감 비교.

### 현재 실행 결과

- `cargo test --locked --manifest-path desktop/backend/Cargo.toml`: `PASS`, library 127 passed·3 ignored, parity 2/2.
- `cargo check --locked --manifest-path desktop/Cargo.toml`: `PASS`.
- `npm run app:dev -- --smoke-test --smoke-report=... --capture=...`: `PASS`, report `passed:true`; main screenshot과 settings roundtrip 기록.
- `cargo test --locked --manifest-path desktop/Cargo.toml`: `PASS`, 8/8.
- `nvm use 22.20.0; npm test`: `PASS`, Node v22.20.0, 213/213.

## Follow-up CP-CPU-CUSTOMIZED-005

- qa_state: `COMPLETE_PASS`
- source_target: `base 60588e80b2002df7e160a4fa469e2301090e2dd9 + owned customized-dropdown target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — standard/custom CPU preset dropdown`
- feature_map: `updated — revision 6`
- architecture_impact: `preserving advanced safe toggle and effective preset projection`
- architecture_contract: `updated — revision 39`
- `SCN-CPU-CUSTOM-001`: `PASS` — main dropdown의 `게임 집중 / 균형 / 안전 모드` DOM 변경이 각각 expected mode/count에 도달했다.
- `SCN-CPU-CUSTOM-002`: `PASS` — advanced 5P 뒤 safe toggle이 count를 보존하고 main dropdown이 네 option과 selected `사용자화`를 표시했다.
- `cargo test --manifest-path desktop/backend/Cargo.toml --locked`: `PASS`.
- `cargo test --manifest-path desktop/Cargo.toml --locked`: `PASS`, 8/8.
- `nvm use 22.20.0; npm test`: `PASS`, 213/213.
- `cargo check --manifest-path desktop/Cargo.toml --locked`: `PASS` after dropdown correction.
- edited-file IDE lints: `PASS`, no diagnostics.
- actual game workload performance: `NOT_RUN`.

## Follow-up CP-CPU-UI-LAYOUT-006

- qa_state: `COMPLETE_PASS`
- source_target: `base 60e83bbb97f86e96d9051f230c2afbaf54757b94 + owned UI-layout target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — independent advanced CPU section and preset pending feedback`
- feature_map: `updated — revision 7`
- architecture_impact: `none — UI composition only`
- architecture_contract: `no_change — revision 39 remains accurate`
- `SCN-CPU-UI-001`: `PASS` — core count와 advanced CPU heading이 서로 다른 direct `.developer-tool-stack`에 있음을 smoke가 확인했다.
- `SCN-CPU-UI-002`: `PASS` — mode pending 상태에서 dropdown value/text/disabled가 `applying / 설정 변경중 / true`였다.
- `SCN-CPU-UI-003`: `PASS` — screenshot에서 border/background 없는 큰 font dropdown과 늘어난 top spacing을 확인했다.
- desktop locked check: `PASS`.
- desktop tests: `PASS`, 8/8.
- main desktop smoke: `PASS`.
- edited-file lints: `PASS`.
- actual game workload: `NOT_RUN`.

## Follow-up CP-CPU-FOCUS-PRIORITY-010

- qa_state: `COMPLETE_PASS for automated source checks; UI smoke BLOCKED`
- source_target: `base 9b7a82ddd0d9bacab0a016d0a531eefe8c2e588e + owned focus-priority/arrow/spacing target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — focus preset priority and selector spacing`
- feature_map: `updated — revision 11`
- architecture_impact: `preset writer and effective projection include priority`
- architecture_contract: `updated — revision 40`
- `BOOST-013`: `PASS` — settings normalization defaults missing focus priority to true while preserving explicit false; preset writer sets focus true and balanced/safe false.
- backend locked tests: `PASS — 127 passed, 3 ignored, parity 2/2`.
- desktop locked tests/check: `PASS — 8/8 and compile`.
- edited-file lints and diff check: `PASS`.
- UI smoke: `BLOCKED — elevated debug app locks the standard executable; alternate target build exhausted temporary disk space and was cleaned`.
- screenshot and actual game workload: `NOT_RUN`.

## Follow-up CP-CPU-DROPDOWN-009

- qa_state: `COMPLETE_PASS`
- source_target: `base 4df74e239ffaad517911a46f62682958337fdeec + owned label/inset target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — boost mode label and compact arrow placement`
- feature_map: `updated — revision 10`
- architecture_impact: `none — visual text and spacing only`
- architecture_contract: `no_change — revision 39 remains accurate`
- `SCN-CPU-DROPDOWN-004`: `PASS` — `부스트 모드 선택`, 110px field, 15px right padding, 6px arrow inset 확인.
- desktop locked check: `PASS`.
- main desktop smoke: first run `FAIL` on unrelated transient WebView2 tab transition; clean-process rerun `PASS`.
- edited-file lints: `PASS`.
- actual game workload: `NOT_RUN`.

## Follow-up CP-CPU-DROPDOWN-008

- qa_state: `COMPLETE_PASS`
- source_target: `base 593bd05a5829b07a951583429308fd35930a450c + owned spacing/copy target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — boost mode chevron/spacing and safe-mode terminology`
- feature_map: `updated — revision 9`
- architecture_impact: `none — visual and user-facing copy only`
- architecture_contract: `no_change — revision 39 remains accurate`
- `SCN-CPU-DROPDOWN-003`: `PASS` — Vulkan gap 10px, ruby gap ≤0px, 1px border chevron을 확인했다.
- `SCN-CPU-ADVANCED-002`: `PASS` — safe-mode description은 `게임 CPU 선호도`를 포함하고 `affinity`를 포함하지 않는다.
- desktop locked check: `PASS`.
- main desktop smoke: `PASS`.
- edited-file lints: `PASS`.
- actual game workload: `NOT_RUN`.

## Follow-up CP-CPU-DROPDOWN-007

- qa_state: `COMPLETE_PASS`
- source_target: `base f8c4411 + owned dropdown/action target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — boost mode presentation and top-level advanced CPU options`
- feature_map: `updated — revision 8`
- architecture_impact: `none — UI composition only`
- architecture_contract: `no_change — revision 39 remains accurate`
- `SCN-CPU-DROPDOWN-001`: `PASS` — computed border `0px`, transparent background, appearance `none`, 11px font, left 9px와 `부스트 모드` label을 확인했다.
- `SCN-CPU-DROPDOWN-002`: `PASS` — standard labels가 `게임 집중 모드 / 균형 모드 / 안전 모드`이고 custom/pending roundtrip이 유지됐다.
- `SCN-CPU-ADVANCED-001`: `PASS` — core count, safe action, priority action, CPU reorder가 direct order로 배치되고 두 option button이 `사용하기`를 표시했다.
- desktop locked check: `PASS`.
- desktop tests: `PASS`, 8/8.
- main desktop smoke: `PASS`.
- edited-file lints: `PASS`.
- actual game workload: `NOT_RUN`.
