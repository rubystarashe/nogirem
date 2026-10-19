# Plan — main-screen CPU presets

- record_id: `PLAN-CPU-MAIN-PRESETS-004`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-MAIN-PRESETS-004`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-19T06:38:00Z`
- target: `REQ-CPU-MAIN-PRESETS-004`
- supersedes: `PLAN-CPU-MODE-SPLIT-003` for final UI composition
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — three main presets and advanced custom count`
- feature_map: `updated — revision 5`
- architecture_impact: `topology computes preset defaults; service writes mode/count; affinity applies hard-binding policy`
- architecture_contract: `updated — revision 38`

## 구현과 검증

1. topology의 mode별 default는 focus=`balanced+2`, balanced/safe=`balanced`이며 manual requested count가 있으면 우선한다.
2. service mode 선택은 persisted custom count를 null로 바꿔 preset default를 복원한다.
3. advanced manual count 변경은 `cpuMode`를 보존한다.
4. main Dioxus home의 Vulkan manager control 아래에 three-button selector를 배치한다.
5. smoke에서 DOM 위치, 세 label, focus 6P, balanced/safe 4P와 advanced manual 5P mode 보존을 확인한다.
6. backend/desktop/Node checks, UI screenshot, SELF_REVIEW와 exact manifest를 기록한다.

`NOTICE.md`, `data/`는 제외하며 실제 게임 frame time은 `NOT_RUN`이다.
