# Checkpoint events

## EVT-CPU-001

- actor: `cursor-agent-92251f36`
- timestamp: `2026-10-19T05:09:00Z`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-TOPOLOGY-PRIORITY-001`
- entity: `DEV`
- previous_state: `NOT_STARTED`
- new_state: `IN_PROGRESS`
- exact_target: `base 1502358cc77df6a35e3515cf5ec98ddd2f0737ab`
- reason: `authorized implementation started`
- evidence: `request.md, plan.md, coordination.md`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — implementation started`
- feature_map: `no_change — intake event`
- architecture_impact: `topology/process/ledger/IPC boundaries`
- architecture_contract: `no_change — intake event`

## EVT-CPU-002

- actor: `cursor-agent-92251f36`
- timestamp: `2026-10-19T05:35:00Z`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-TOPOLOGY-PRIORITY-001`
- entity: `QA`
- previous_state: `IN_PROGRESS`
- new_state: `COMPLETE_PASS`
- exact_target: `EVID-CPU-TOPOLOGY-SOURCE-001`
- reason: `mandatory automated source scenarios passed; actual game scenario explicitly NOT_RUN`
- evidence: `qa.md`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — automated behavior verified`
- feature_map: `updated — revision 31`
- architecture_impact: `topology/process/ledger/IPC boundaries verified`
- architecture_contract: `updated — revision 35`

## EVT-CPU-004

- actor: `cursor-agent-92251f36`
- timestamp: `2026-10-19T06:10:00Z`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-MODES-002`
- entity: `DEV`
- previous_state: `DONE`
- new_state: `IN_PROGRESS`
- exact_target: `base 17dee9063db8c9926d70a1f9eab5a4955b7c47d8`
- reason: `사용자가 게임 집중·균형·안전 mode와 공격적 P-core preset을 후속 승인`
- evidence: `request-mode-followup.md, mode-plan.md`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — mode follow-up started`
- feature_map: `no_change — intake event`
- architecture_impact: `topology mode and settings IPC`
- architecture_contract: `no_change — intake event`

## EVT-CPU-005

- actor: `cursor-agent-92251f36`
- timestamp: `2026-10-19T06:27:00Z`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-MODES-002`
- entity: `QA and review`
- previous_state: `IN_PROGRESS / NOT_REVIEWED`
- new_state: `COMPLETE_PASS / APPROVED`
- exact_target: `follow-up owned-file target; final manifest pending`
- reason: `mode fixtures, full backend/desktop/Node checks and desktop smoke passed; SELF_REVIEW findings resolved`
- evidence: `qa.md, reviews/mode-self-review.md`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — mode source and automated behavior verified`
- feature_map: `updated — revision 3`
- architecture_impact: `mode authority and live control verified; ledger unchanged`
- architecture_contract: `updated — revision 36`

## EVT-CPU-006

- actor: `cursor-agent-92251f36`
- timestamp: `2026-10-19T06:37:00Z`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-MAIN-PRESETS-004`
- entity: `scope`
- previous_state: `CP-CPU-MODE-SPLIT-003`
- new_state: `CP-CPU-MAIN-PRESETS-004`
- exact_target: `base 17dee9063db8c9926d70a1f9eab5a4955b7c47d8`
- reason: `사용자가 게임 집중·균형 preset 유지와 main Vulkan control 아래 placement를 정정`
- evidence: `request-main-presets.md, main-presets-plan.md`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — final preset composition and placement`
- feature_map: `updated — revision 5`
- architecture_impact: `preset orchestration and main UI`
- architecture_contract: `updated — revision 38`

## EVT-CPU-007

- actor: `cursor-agent-92251f36`
- timestamp: `2026-10-19T06:49:00Z`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-MAIN-PRESETS-004`
- entity: `QA and review`
- previous_state: `IN_PROGRESS / NOT_REVIEWED`
- new_state: `COMPLETE_PASS / APPROVED`
- exact_target: `final owned-file target; manifest pending`
- reason: `backend/desktop/Node checks and main UI smoke passed; mandatory SELF_REVIEW findings resolved`
- evidence: `qa.md, reviews/main-presets-self-review.md, cpu-presets-smoke.json`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — final presets verified in automated scope`
- feature_map: `updated — revision 5`
- architecture_impact: `topology/settings/worker/UI boundaries verified`
- architecture_contract: `updated — revision 38`

## EVT-CPU-003

- actor: `cursor-agent-92251f36`
- timestamp: `2026-10-19T05:35:00Z`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-TOPOLOGY-PRIORITY-001`
- entity: `review`
- previous_state: `NOT_REVIEWED`
- new_state: `APPROVED`
- exact_target: `EVID-CPU-TOPOLOGY-SOURCE-001`
- reason: `SELF_REVIEW resolved all mandatory findings`
- evidence: `reviews/self-review.md`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — exact diff reviewed`
- feature_map: `updated — revision 31`
- architecture_impact: `no unresolved new architecture violations`
- architecture_contract: `updated — revision 35`
