# Graphics routing and AMD restore plan

- record_id: `PLAN-GRAPHICS-RESTORE-001`
- run_id: `RUN-20261015-GRAPHICS-ROUTING-AMD-RESTORE`
- checkpoint_id: `CP-GRAPHICS-RESTORE-001`
- owner: `cursor-agent-92251f36`
- updated_at: `2026-10-15T13:35:00Z`
- target: `main base 061a23a4c7a6fc977ce8b3ec32e43c691ab8d1c0 + owned working tree`
- status: `IN_PROGRESS — independent review pending`
- feature_impact: `FEAT-NOGIREM-GRAPHICS-SETTINGS — mixed GPU routing and AMD optimization`
- feature_map: `updated — revision 2`
- architecture_impact: `NVIDIA-first selection and AMD detect/apply separation`
- architecture_contract: `updated — revision 29`

## Checkpoints

1. `completed` — 실제 WMI와 CLI로 RTX 4090 + AMD 내장 GPU 오선택 원인 확인.
2. `completed` — NVIDIA 우선 routing과 AMD WMI-only 상태 응답 구현.
3. `completed` — Radeon `I3DSettings` 적용 복구, incompatible init·legacy retry 제거, tuning contract 강제.
4. `completed` — Dioxus UI와 Electron compatibility 상태 조회를 helper 미호출 경로로 정렬.
5. `completed` — helper rebuild, Rust·Node·desktop·실제 NVIDIA status 검증.
6. `in_progress` — independent review, exact target manifest, run/handoff 정리와 local commit.

## 설계

- canonical_path: `UI → graphics service → graphics.rs → NVIDIA NVAPI 또는 AMD WMI/explicit helper`
- NVIDIA: 상태 확인이 성공하면 즉시 반환하며 AMD adapter를 호출하지 않는다.
- AMD status: WMI vendor ID와 이름만 읽고 목표를 `pending`으로 표시한다.
- AMD apply: 사용자 확인 후 helper의 네 3D 설정과 게임 VSync만 적용한다.
- writer: NVAPI DRS와 ADLX `I3DSettings`가 각각 해당 vendor 설정의 유일 writer다.
- prohibited: GPU tuning·performance API, incompatible driver initialization, startup/refresh ADLX, contract 없는 helper output.
- partial_failure: helper는 네 3D 설정을 적용 전에 snapshot하고 항목 실패 시 전체 snapshot 복원을 시도한다. reason이 있으면 caller는 게임 VSync를 쓰지 않고 최종 상태를 오류에 포함한다.
- validation: graphics unit, full backend/parity, Radeon Node, full Node, desktop compile, helper build, actual mixed-GPU CLI status, diff/lint, independent review.
- stop_condition: must-fix finding 0, 자동 검증 통과, 정확한 변경 파일만 commit, AMD hardware limitation 명시.
