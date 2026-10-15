# RUN-20261015-AMD-SETTINGS-PROTECTION

- run_id: `RUN-20261015-AMD-SETTINGS-PROTECTION`
- checkpoint_id: `CP-AMD-PROTECTION-001`
- owner: `cursor-agent-92251f36`
- roles: `Coordinator=DEV=QA=Documentation maintainer cursor-agent-92251f36; Reviewer=generalPurpose agent 800281e8-c046-4ad3-94cd-ca609578ca6c`
- created_at: `2026-10-15T12:27:00Z`
- updated_at: `2026-10-15T12:50:00Z`
- base_target: `nogirem main da014021ccbb122bb1897ec895da48292669b777`
- source_target: `uncommitted owned changes in desktop/backend/src/graphics.rs, desktop/src/ui.rs and this run's canonical documentation`
- status: `READY_FOR_COMMIT`
- DEV_state: `READY_FOR_REVIEW`
- QA_state: `COMPLETE_PASS`
- review_state: `APPROVED`
- feature_impact: `FEAT-NOGIREM-GRAPHICS-SETTINGS — AMD 장치가 존재하면 ADLX 상태 조회와 전역 설정 적용을 모두 차단하고 읽기 전용 보호 상태를 표시`
- feature_map: `updated — 그래픽 설정 기능 identity와 AMD 보호 동작을 canonical map에 추가`
- architecture_impact: `그래픽 상태 조회의 Radeon 경계를 ADLX helper에서 Win32_VideoController VEN_1002 읽기 전용 탐지로 변경`
- architecture_contract: `updated — AMD 장치 존재 시 드라이버 설정 API 금지와 혼합 GPU 보호 우선순위 명시`

## 요청과 진단

- original_request: `렘 부스터를 키면 AMD 언더볼팅 셋팅한 값이 날라갑니다`와 진단 ZIP `nogirem-diagnostics-0.4.2-20261002-003254-d5cf1c73-7df9-4f22-8c82-efe78071130e - 진하울.zip`
- objective: 앱 시작·상태 새로고침·최적화 적용이 AMD 사용자 튜닝 프로필에 접근하지 않도록 한다.
- diagnostic_target: 앱 `0.4.2`, AMD Radeon RX 6700 XT, driver `32.0.21045.5002`
- source_finding: v0.4.2와 현재 source는 `apply=false` 상태 조회에서도 non-NVIDIA 시스템의 `radeon-helper.exe`를 실행해 ADLX를 초기화·종료했다. `ResetToFactory`나 GPU tuning API 호출은 없었으며 실제 쓰기는 명시적 최적화에서 3D 전역 설정 네 개에만 존재했다.
- causal_limit: 공식 ADLX 계약상 초기화 자체가 튜닝을 초기화하지는 않으므로 드라이버 내부의 정확한 초기화 원인은 진단 ZIP만으로 확정하지 못했다. 제보 증상과 자동 ADLX 조회 시점이 일치해 제품 경계에서 호출 자체를 제거했다.
- scope: Rust 그래픽 backend의 AMD 탐지·적용 정책, Dioxus 그래픽 상태 UI, 회귀 테스트, canonical 기능·아키텍처·handoff.
- exclusions: NVIDIA 전용 시스템의 기존 최적화, Radeon 드라이버 자체 수정, 기존 사용자 튜닝값 복구, package·push·release·deployment.
- risk: `moderate — GPU 드라이버 설정 경계를 변경하지만 AMD에서는 모든 설정 API를 제거하고 장치 식별 읽기만 수행한다. durable data·schema·network·installer 영향은 없고 두 source 파일에 국한된다. 실제 제보 RX 6700 XT의 언더볼팅 유지 수동 검증은 필요하다.`

## 구현

- `Win32_VideoController`에서 PCI vendor `VEN_1002`만 읽어 AMD 장치 존재와 표시 이름을 정규화한다.
- AMD 장치가 하나라도 있으면 혼합 GPU에서도 보호를 우선하고 NVIDIA NVAPI와 Radeon ADLX helper를 호출하지 않는다.
- AMD 적용 요청은 어떤 드라이버 write 전에도 오류로 거부한다.
- 장치 이름이 null 또는 빈 문자열이어도 장치 존재를 유지하고 `AMD Radeon GPU`로 표시한다.
- AMD WMI 조회가 실패한 NVIDIA 전용 시스템은 기존 NVIDIA 경로를 계속 사용할 수 있다.
- UI는 `AMD Radeon 설정 보호`, `설정 보호됨`, 읽기·쓰기 차단 안내를 표시하며 기존 AMD 적용 모달도 보호 안내만 제공한다.

## QA와 증거

- `cargo test --locked --manifest-path desktop/backend/Cargo.toml graphics`: `PASS — 6/6`
- `cargo test --locked --manifest-path desktop/backend/Cargo.toml`: `PASS — 111/111, 3 declared ignored, integration parity 2/2 PASS`
- `npm test`: `PASS — 206/206`
- `cargo check --locked --manifest-path desktop/Cargo.toml`: `PASS`
- `cargo run --quiet --locked --manifest-path desktop/backend/Cargo.toml --bin nogirem-cli -- --graphics-status`: `PASS — 실제 AMD 장치를 read-only/writeProtected 상태로 반환`
- IDE diagnostics: `PASS — 변경 source 오류 없음`
- manual_product_QA: `NOT_RUN — 제보 RX 6700 XT에서 앱 시작 전후 언더볼팅 값 비교는 사용자 환경 필요`
- execution_environment: `Windows 10.0.26200 x64, local source checkout`
- dependencies: `실제 상태 실행은 Windows WMI를 사용했으며 Radeon ADLX helper와 GPU 설정 API는 호출하지 않음`

## 리뷰

- review_id: `REV-AMD-PROTECTION-001`
- review_mode: `INDEPENDENT_REVIEW`
- reviewer: `generalPurpose agent 800281e8-c046-4ad3-94cd-ca609578ca6c`
- round_1: `CHANGES_REQUESTED — ISSUE-GFX-001 혼합 GPU 보호 우선순위, ISSUE-GFX-002 실행 경로 테스트, ISSUE-GFX-003 UI 계약`
- round_2: `CHANGES_REQUESTED — ISSUE-GFX-004 이름 누락 장치 탐지, ISSUE-GFX-005 WMI 실패 시 NVIDIA 회귀`
- final_verdict: `APPROVED — ISSUE-GFX-001~005 RESOLVED, 신규 must_fix 없음`
- approval_freshness: `현재 source 변경 이후 재검토 완료`

## 완료 게이트와 handoff

- feature_gate: `PASS for source accuracy; BLOCKED for behavioral verification on the reported RX 6700 XT`
- architecture_gate: `PASS — AMD driver setting API 경로 제거, 신규 writer·예외 없음`
- review_gate: `PASS — 독립 리뷰 APPROVED`
- QA_gate: `PASS for automated checks; manual_product_QA NOT_RUN`
- target_gate: `PENDING — 최종 manifest와 local commit 필요`
- final_disposition: `PARTIALLY_COMPLETED — source 수정과 자동 검증은 완료, 제보 하드웨어 수동 확인은 남음`
- push_status: `NOT_RUN — 승인되지 않음`
- deployment_status: `NOT_RUN — 승인되지 않음`
- next_authorized_action: `owned-file manifest 확인과 local commit 생성`
