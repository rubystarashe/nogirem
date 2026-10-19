# Follow-up — 0.4.8 CPU verification and version history

- record_id: `TASK-CPU-VERIFY-048-011`
- run/checkpoint: `20261019-cpu-topology-priority / CP-CPU-VERIFY-048-011`
- owner/roles: `cursor-agent-92251f36 — Coordinator, DEV-as-QA, Documentation maintainer, SELF_REVIEW`
- created_at: `2026-10-19T08:42:00Z`
- updated_at: `2026-10-19T08:45:00Z`
- source_target: `e988317bf87e49b192c80dad76b32dcb0657ae84`
- status: `COMPLETED`
- DEV_state: `DONE`
- QA_state: `COMPLETE_PASS for source/topology/process scenarios`
- review_state: `APPROVED`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — 기존 CPU topology·preset·priority 동작 재검증과 0.4.8 사용자 변경 기록`
- feature_map: `no_change — revision 11의 기능 범위와 제한이 현재 source에 정확함`
- architecture_impact: `none — topology/settings/process 경계를 재검토했으며 source 구조는 변경하지 않음`
- architecture_contract: `no_change — revision 40이 focus priority preset과 fail-safe 경계를 정확히 설명함`

## 요청과 범위

- original_request: `L3기반 선택 등 이번에 변경된 구성들이 실제로 제대로 동작했는지 다시 체크하고, 0.4.8 버전 변경사항으로 기록해`
- objective: 현재 commit에서 L3 locality, SMT, P/E 분류, SchedulingClass, focus/balanced count, hybrid P-core 공유, 저코어 background 보존, processor-group fail-safe, priority role·복원과 focus preset 기본값을 다시 확인하고 검증된 사용자 영향만 `VERSION_HISTORY.md`의 0.4.8 항목으로 기록한다.
- exclusions: crate/package 버전 변경, package·release·push·deploy, 실제 마비노기 frame-time 비교, 복수 processor group affinity 구현.
- risk: `LOW — 제품 source를 변경하지 않고 기존 자동 검증을 재실행하며 사용자 변경 이력만 추가한다. durable data·보안·통합 경계 변화가 없다.`

## Source review

- `GetSystemCpuSetInformation`의 `EfficiencyClass`로 P/E 소속을 판별하고 `SchedulingClass`는 같은 P-core 후보의 정렬에만 사용한다.
- `GetLogicalProcessorInformationEx`의 processor core·L3 cache·group mask를 CPU Set과 결합하고 물리 코어의 모든 SMT logical processor를 하나의 mask로 유지한다.
- game core는 가장 큰 L3 domain을 먼저 사용하며 같은 domain 안에서 높은 SchedulingClass와 낮은 logical index 순으로 선택한다.
- non-hybrid는 background 물리 코어를 하나 이상 남기고 hybrid에서 모든 P-core를 game에 주면 마지막 game P-core 하나를 background와 공유한다.
- 복수 processor group은 현재 완전 지원하지 않고 단일 `u64` mask 오적용을 막기 위해 CPU 분할을 비활성화한다.
- priority 하향은 일반 background role에만 적용하고 game·latency-sensitive·excluded role에는 적용하지 않는다. PID+create time identity와 current-equals-applied 조건으로 원래 affinity/priority를 복원한다.
- focus preset은 priority 하향 true, balanced/safe는 false를 저장한다. 명시적으로 저장된 focus false는 보존되어 사용자화로 표시된다.

## QA evidence

- `cargo test --locked --manifest-path desktop/backend/Cargo.toml topology::tests -- --nocapture`: `PASS — 9/9`.
  - L3 locality, SMT/non-SMT, P/E 분리, SchedulingClass, 8P focus 6-core, 4-core background 보존, all-P hybrid 공유, multi-group fail-safe, malformed buffer를 확인했다.
  - current Windows 10.0.26200 x64 / Ryzen 7 9800X3D에서 live API 조회가 성공하고 game/background mask가 nonzero이며 합집합이 active mask와 같았다.
- `affinity::tests`: `PASS — 5/5`.
- `process::tests::child_priority_roundtrip_uses_one_verified_handle`: `PASS — 1/1`, 실제 child process priority 적용·복원.
- `settings::tests::cpu_modes_default_to_focus_and_migrate_legacy_settings`: `PASS — 1/1`.
- prior exact source evidence: backend `127 PASS·3 ignored`, parity `2/2`, desktop `8/8`, locked check.
- command correction: 여러 Cargo test filter를 한 명령에 전달한 최초 호출은 CLI 사용 오류로 exit 1이었다. 각 filter를 독립 실행해 모두 통과했으며 제품 실패로 분류하지 않는다.
- actual Mabinogi frame/input and multi-group hardware: `NOT_RUN`.

## Documentation and review

- `VERSION_HISTORY.md`에 `0.4.8` 항목을 추가해 검증된 CPU topology, 세 preset, focus priority, background 배정, fail-safe와 사용자화 UI를 기록했다.
- crate/package 버전은 `0.4.7` 그대로이며 0.4.8 package나 release를 생성하지 않았다.
- review_mode: `SELF_REVIEW`; 독립 reviewer 없음.
- review_scope: test 결과와 source algorithm을 버전 기록 각 문장과 대조하고, multi-group을 지원 완료로 오기하지 않았는지 확인했다.
- findings: mandatory finding 없음.
- verdict: `APPROVED`.
- final_disposition: `COMPLETED — source 검증과 0.4.8 변경 기록 완료`
- push/deploy/release: `NOT_RUN — 승인되지 않음`
