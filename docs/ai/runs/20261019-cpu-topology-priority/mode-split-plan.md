# Plan — manual count and affinity mode split

- record_id: `PLAN-CPU-MODE-SPLIT-003`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-MODE-SPLIT-003`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-19T06:30:00Z`
- target: `REQ-CPU-MODE-SPLIT-003`
- supersedes: `PLAN-CPU-MODES-002`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — independent manual count and focus/safe selection`
- feature_map: `updated — revision 4`
- architecture_impact: `topology owns count mask; affinity manager owns whether game mask is applied`
- architecture_contract: `updated — revision 37`

## 구현

1. topology는 mode를 알지 않고 persisted/requested P-core 수만 계산한다.
2. `EfficiencyClass`만 P/E membership을 결정하고 `SchedulingClass`는 동일 P-core 후보 정렬에 사용한다.
3. `gameCoreCount`와 `cpuMode=focus|safe`를 독립 저장하고 모든 live control payload에 두 값을 함께 전달한다.
4. UI에 클릭 가능한 core count row와 별도 두 mode selector를 제공한다.
5. smoke fixture에서 6P 선택 뒤 safe 전환 시 6P가 보존되는지 검증한다.
6. 원 `plan.md`, `mode-plan.md`는 당시 요청의 불변 기록으로 유지한다.

## 검증

- backend topology/settings/worker tests, parity, desktop check/tests, Node service contract, desktop smoke와 screenshot.
- 실제 마비노기 frame time은 `NOT_RUN`.
- `NOTICE.md`, `data/`는 commit에서 제외한다.
- review_mode: `SELF_REVIEW`.
