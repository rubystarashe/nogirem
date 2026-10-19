# SELF_REVIEW — CPU allocation modes

- record_id: `REV-CPU-MODES-002`
- round_id: `REV-CPU-MODES-002-R1`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-MODES-002`
- reviewer: `cursor-agent-92251f36`
- review_mode: `SELF_REVIEW`
- independent_reviewer: `none`
- created_at: `2026-10-19T06:27:00Z`
- target: `base 17dee9063db8c9926d70a1f9eab5a4955b7c47d8 + follow-up owned-file target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — mode semantics and P-core allocation`
- feature_map: `updated — revision 3`
- architecture_impact: `topology authority, CPU setting writer and live worker control`
- architecture_contract: `updated — revision 36`

## 검토 범위

`topology.rs`, affinity manager/worker, settings, Boost/service contract, Dioxus UI/CSS, smoke fixture와 관련 tests를 실제 diff 및 주변 source와 함께 검토했다. durable ledger와 process-handle restore 경계가 mode 변경으로 우회되지 않는지도 확인했다.

## Findings와 DEV response

- `ISSUE-MODE-001`, severity `high`, must_fix `true`, state `RESOLVED`: 모든 P-core를 게임에 선택한 hybrid에서 단순 complement는 background를 E-core에만 남긴다. DEV가 선택된 물리 P-core 하나의 전체 SMT mask를 background에 명시적으로 공유하고 2P+4E fixture를 추가했다.
- `ISSUE-MODE-002`, severity `medium`, must_fix `true`, state `RESOLVED`: background priority만 변경할 때 legacy custom `gameCoreCount`가 실행 중 worker에서 자동값으로 사라질 수 있었다. DEV가 control payload에 persisted count를 포함해 priority 변경과 mode 변경을 분리했다.
- `ISSUE-MODE-003`, severity `medium`, must_fix `true`, state `RESOLVED`: 세 preset과 직접 코어 수 button을 동시에 노출하면 mode 의미가 불명확하다. DEV가 mode button만 조작 가능하게 하고 코어 button은 현재 계산 결과 시각화로 제한했다.

## Verdict

- verdict: `APPROVED`
- unresolved_mandatory_findings: `none`
- stale: `false`
- limitations: 동일 actor가 구현·QA·review를 수행한 `SELF_REVIEW`이며 실제 마비노기 성능 비교는 참여하지 않았다.
