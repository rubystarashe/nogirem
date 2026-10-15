# Graphics routing and AMD restore QA

- record_id: `QA-GRAPHICS-RESTORE-001`
- run_id: `RUN-20261015-GRAPHICS-ROUTING-AMD-RESTORE`
- checkpoint_id: `CP-GRAPHICS-RESTORE-001`
- owner: `cursor-agent-92251f36 acting as DEV-as-QA`
- executed_at: `2026-10-15T13:31:00Z–2026-10-15T13:41:00Z`
- source_target: `base 061a23a4c7a6fc977ce8b3ec32e43c691ab8d1c0 + current owned working tree`
- execution_target: `Windows 10.0.26200 x64 local debug source; NVIDIA driver 610.88; RTX 4090 + AMD Radeon(TM) Graphics`
- QA_state: `COMPLETE_PASS for mandatory available automation; AMD hardware manual scenario NOT_RUN`
- feature_impact: `FEAT-NOGIREM-GRAPHICS-SETTINGS — routing and AMD apply behavior`
- feature_map: `updated — revision 2`
- architecture_impact: `driver adapter selection and tuning exclusion`
- architecture_contract: `updated — revision 29`

## 실행 결과

- `GFX-007 mixed GPU NVIDIA priority`: `PASS` — 실제 `nogirem-cli --graphics-status`가 `NVIDIA GeForce RTX 4090`, `vendor=nvidia`, `title=NVIDIA 프로필`을 반환했다.
- `GFX-001 AMD status without ADLX`: `PASS` — Rust unit과 Electron Node test에서 WMI 상태가 네 pending 목표를 반환하고 helper call count는 0이다.
- `GFX-008 AMD explicit apply routing`: `PASS` — apply=true만 helper로 전달되고 네 목표를 snapshot한다. 항목 실패 시 snapshot 복원을 시도하며 helper reason이 있으면 게임 VSync를 쓰지 않는다.
- `GFX-009 incompatible initialization forbidden`: `PASS` — helper source에 `InitializeWithIncompatibleDriver`가 없고 Node는 실패 시 단 한 번만 호출한다.
- `GFX-010 tuning contract`: `PASS` — helper는 `tuningAccessed:false`; Rust·Electron은 누락 결과를 거부한다.
- helper build: `PASS` — `npm run native:radeon`, MSVC Release output SHA-256 `e1d3a09a9edf14be4f4914fbcfb4aab54d102b0f75edd83402ed6caae47643e8`.
- Rust graphics: `PASS — 6/6`.
- full backend: `PASS — 115 passed, 3 declared ignored`; legacy/native parity `2/2 PASS`.
- Radeon Node: `PASS — 13/13`.
- full Node: `PASS — 211/211`.
- desktop compile: `PASS`.
- diff check and IDE diagnostics: `PASS`.
- `GFX-AMD-HARDWARE-APPLY`: `NOT_RUN — AMD 단독 GPU 장비와 driver tuning 관측 수단이 현재 실행 환경에 없음`.

## 범위와 한계

- 실제 혼합 GPU source routing은 수동으로 실행했다.
- 사용자 언더볼팅·오버클럭 값은 수집하지 않았고 evidence에 포함하지 않았다.
- source가 tuning API를 호출하지 않는 것은 확인했지만 AMD driver 내부가 정상 ADLX 초기화 중 tuning 값을 보존하는지는 자동 검사로 증명할 수 없다.
- package·설치·게임 실행·배포는 수행하지 않았다.
