# RUN-20261015-GRAPHICS-ROUTING-AMD-RESTORE

- run_id: `RUN-20261015-GRAPHICS-ROUTING-AMD-RESTORE`
- checkpoint_id: `CP-GRAPHICS-RESTORE-001`
- owner: `cursor-agent-92251f36`
- roles: `Coordinator=DEV=QA=Documentation maintainer cursor-agent-92251f36; Reviewer=generalPurpose agent adf951c0-1cb0-4463-8c6b-2751a0e957fe`
- created_at: `2026-10-15T13:28:00Z`
- updated_at: `2026-10-15T13:35:00Z`
- base_target: `nogirem main 061a23a4c7a6fc977ce8b3ec32e43c691ab8d1c0`
- source_target: `commit d1b87085c360133eab49455348142efe94435f78`
- status: `COMPLETED`
- DEV_state: `DONE`
- QA_state: `COMPLETE_PASS for automated checks; AMD hardware scenario NOT_RUN`
- review_state: `APPROVED — R3 exact code hashes, ISSUE-GFX-R-001~003 RESOLVED`
- feature_impact: `FEAT-NOGIREM-GRAPHICS-SETTINGS — 혼합 GPU NVIDIA 선택과 AMD Radeon 3D 최적화 복구`
- feature_map: `updated — revision 2와 GFX-007~010`
- architecture_impact: `NVIDIA 우선 routing, AMD 상태 조회의 ADLX 미호출, 명시 적용의 I3DSettings-only helper 계약`
- architecture_contract: `updated — revision 29`

## 요청과 범위

- original_request: 현재 대화의 혼합 GPU 오탐 수정과 AMD 최적화 기능 복구 요청. AMD에서도 언더볼팅이 해제되지 않도록 요구했다.
- objective: RTX 4090과 AMD 내장 GPU가 함께 있는 시스템은 NVIDIA 프로필을 사용하고, AMD 단독 시스템은 Radeon 3D 최적화를 다시 제공하되 tuning API를 사용하지 않는다.
- acceptance_criteria:
  1. NVIDIA가 감지되면 AMD WMI와 Radeon helper보다 먼저 NVIDIA 프로그램 profile을 반환한다.
  2. AMD 단독 상태 조회는 ADLX helper를 시작하지 않고 GPU와 네 최적화 목표를 표시한다.
  3. 사용자 확인 뒤 AMD 수직 동기화·Enhanced Sync·Anti-Lag·Chill 적용을 수행한다.
  4. Radeon helper는 `I3DSettings`만 사용하고 GPU tuning API와 `InitializeWithIncompatibleDriver`를 사용하지 않는다.
  5. Rust와 Electron 호출자는 `tuningAccessed:false` 계약이 없는 helper 결과를 거부한다.
- exclusions: 사용자 tuning 값을 읽거나 저장·복원하는 기능, AMD driver 내부 동작 수정, package·push·release·deployment.
- risk: `moderate — GPU driver 전역 3D 설정 writer를 복구한다. 상태 조회와 적용을 분리하고 tuning API와 비호환 강제 초기화를 금지했지만 실제 AMD driver가 ADLX 정상 초기화 중 사용자 tuning을 보존하는지는 해당 하드웨어 수동 검증이 필요하다. durable application data·schema·network 영향은 없다.`
- authorization: source·test·helper rebuild·canonical docs·local commit. package·외부 게시·배포는 승인되지 않았다.

## 구현과 결정

- `graphics.rs`가 NVIDIA status를 먼저 확인하며 실제 혼합 시스템에서 RTX 4090을 선택한다.
- NVIDIA가 없을 때만 AMD WMI 장치 탐지를 수행한다. `apply=false`는 ADLX를 열지 않고 네 목표를 `적용 전`으로 반환한다.
- AMD 명시 적용만 rebuilt `radeon-helper.exe`를 실행하고 결과를 정규화·검증한다.
- helper의 legacy/incompatible driver 초기화와 Electron 재시도를 제거했다.
- helper source는 `I3DSettings.h`만 사용하며 결과에 `tuningAccessed:false`를 포함한다. Rust와 Electron은 해당 필드가 없거나 true이면 중단한다.
- helper는 모든 AMD GPU의 네 3D 설정을 먼저 snapshot하고 적용 실패 시 snapshot 복원을 시도한다. 실패 reason이 있으면 Rust·Electron은 게임 VSync를 변경하지 않고 최종 Radeon 상태를 오류에 포함한다.
- UI는 읽기 전용 보호 카드를 제거하고 AMD 최적화 확인·적용 흐름과 tuning 미접근 안내를 복구했다.
- canonical_extension_path: `desktop/src/ui.rs → service.rs → graphics.rs → native/radeon-helper I3DSettings`
- durable_writer: `AMD Radeon driver global 3D settings writer=native/radeon-helper explicit apply only`; tuning·전압·클럭 writer는 없다.
- compatibility: 0.3.x Electron compatibility path도 비호환 driver retry와 계약 없는 helper 결과를 거부한다.

## QA

- `npm run native:radeon`: `PASS — MSVC Release helper rebuilt`
- `cargo test --locked --manifest-path desktop/backend/Cargo.toml graphics`: `PASS — 6/6`
- `cargo test --locked --manifest-path desktop/backend/Cargo.toml`: `PASS — 115 passed, 3 declared ignored, parity 2/2`
- `node --test test/radeon.test.mjs`: `PASS — 13/13`
- `npm test`: `PASS — 211/211`
- `cargo check --locked --manifest-path desktop/Cargo.toml`: `PASS`
- `cargo run --quiet --locked --manifest-path desktop/backend/Cargo.toml --bin nogirem-cli -- --graphics-status`: `PASS — NVIDIA GeForce RTX 4090, vendor=nvidia, title=NVIDIA 프로필`
- `git diff --check`: `PASS`
- IDE diagnostics: `PASS — 변경 source 오류 없음`
- `GFX-AMD-HARDWARE-APPLY`: `NOT_RUN — AMD 단독 GPU와 적용 전후 tuning 값을 확인할 실행 환경이 없음`
- execution_environment: `Windows 10.0.26200 x64, RTX 4090 + AMD Radeon(TM) Graphics, local source checkout`
- evidence_limit: 실제 source 실행으로 혼합 GPU routing은 확인했지만 rebuilt helper를 이 혼합 시스템에서 강제로 실행하지 않아 AMD 적용과 tuning 보존은 자동 검사·source boundary만 검증했다.

## 리뷰와 현재 게이트

- review_id: `REV-GRAPHICS-RESTORE-001`
- review_mode: `INDEPENDENT_REVIEW`
- reviewer: `generalPurpose agent adf951c0-1cb0-4463-8c6b-2751a0e957fe`
- review_target: `base 061a23a4c7a6fc977ce8b3ec32e43c691ab8d1c0 + current working tree`
- review_verdict: `APPROVED — no unresolved must-fix`
- feature_gate: `PASS for source accuracy and mixed-GPU behavior; AMD hardware behavior remains NOT_RUN`
- architecture_gate: `PASS — tuning API and incompatible initialization mechanically excluded; snapshot rollback verified by source review`
- QA_gate: `PASS for automated and mixed-GPU source execution; AMD explicit apply NOT_RUN`
- target_gate: `PASS — exact code hashes reviewed and committed as d1b87085c360133eab49455348142efe94435f78`
- final_disposition: `COMPLETED — authorized source scope committed; AMD hardware manual QA remains a disclosed non-blocking limitation`
- push_status: `NOT_RUN — unauthorized`
- deployment_status: `NOT_RUN — unauthorized`

## Exact reviewed code target

- `desktop/backend/src/graphics.rs`: `55110c60e9528715cfaa6258e1602f3d47dff2af12206e18c74b4824f96ae6a9`
- `desktop/src/ui.rs`: `fd070ee9b3f15c0d908d573181ae3e14f3f8ac28e8f0e161a0ac2fa749076f1c`
- `native/radeon-helper/main.cpp`: `2292adc6fa5234f7e1bb05b698d602f9eb871059060f52b5f8be80c7ab9248c8`
- `native/radeon-helper/bin/radeon-helper.exe`: `e1d3a09a9edf14be4f4914fbcfb4aab54d102b0f75edd83402ed6caae47643e8`
- `src/radeon.mjs`: `e2d608572077a5c8b94687bb491e18b35f963b02614f200e3c673ce750f9a702`
- `test/radeon.test.mjs`: `d67bcbf3fe25787204eab03cdd8ffde42376925a9bffc6a726a19ca4264db6dd`
