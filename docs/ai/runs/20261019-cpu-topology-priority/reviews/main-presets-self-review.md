# SELF_REVIEW — main CPU presets

- record_id: `REV-CPU-MAIN-PRESETS-004`
- round_id: `REV-CPU-MAIN-PRESETS-004-R1`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-MAIN-PRESETS-004`
- reviewer: `cursor-agent-92251f36`
- review_mode: `SELF_REVIEW`
- independent_reviewer: `none`
- created_at: `2026-10-19T06:49:00Z`
- target: `base 17dee9063db8c9926d70a1f9eab5a4955b7c47d8 + final owned-file target`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — main three presets and advanced manual count`
- feature_map: `updated — revision 5`
- architecture_impact: `preset defaults, topology classification, settings writer and UI placement`
- architecture_contract: `updated — revision 38`

## 검토 범위

Topology membership/selection, mode defaults, custom count persistence, live worker rebuild, legacy migration, main/advanced UI separation, service fixture, smoke와 CSS를 실제 diff와 주변 source에서 검토했다. process ledger와 priority role exclusion은 변경되지 않았음을 확인했다.

## Findings와 DEV response

- `ISSUE-PRESET-001`, severity `high`, must_fix `true`, state `RESOLVED`: `(EfficiencyClass, SchedulingClass)` 전체 쌍으로 P-core membership을 판정하면 homogeneous CPU의 preferred-core SchedulingClass 차이가 나머지 P-core를 제외할 수 있다. DEV가 membership은 EfficiencyClass만 사용하고 SchedulingClass는 후보 정렬에만 사용하도록 수정하고 fixture를 추가했다.
- `ISSUE-PRESET-002`, severity `high`, must_fix `true`, state `RESOLVED`: mode selector가 고급 기능 안에 있으면 최종 사용자 요청의 main quick preset 진입점을 충족하지 않는다. DEV가 Vulkan 관리 control 바로 아래 fixed selector로 이동하고 smoke에서 실제 bounding position을 검증했다.
- `ISSUE-PRESET-003`, severity `medium`, must_fix `true`, state `RESOLVED`: preset과 manual count가 함께 존재할 때 override 규칙이 불명확했다. DEV가 advanced count는 mode를 보존하고 main preset은 custom count를 제거해 자동값으로 복귀하는 single-writer 규칙을 구현했다.
- `ISSUE-PRESET-004`, severity `medium`, must_fix `true`, state `RESOLVED`: hybrid all-P complement는 background를 E-core-only로 만들 수 있다. 한 물리 P-core의 전체 SMT mask만 공유하고 fixture로 고정했다.

## Verdict

- verdict: `APPROVED`
- unresolved_mandatory_findings: `none`
- stale: `false`
- limitations: 동일 actor가 구현·QA·review를 수행한 `SELF_REVIEW`; 실제 마비노기 frame·입력 성능은 검토 evidence에 포함되지 않는다.
