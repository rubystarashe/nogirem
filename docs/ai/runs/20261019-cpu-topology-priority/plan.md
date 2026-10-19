# Plan — CPU topology and priority

- record_id: `PLAN-CPU-TOPOLOGY-PRIORITY-001`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-TOPOLOGY-PRIORITY-001`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-19T05:09:00Z`
- target: `request.md revision 1`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — topology-aware non-overlapping allocation and advanced affinity/priority controls`
- feature_map: `updated — completion checkpoint`
- architecture_impact: `topology model, Windows process adapter, affinity ledger, helper control and service/UI settings`
- architecture_contract: `updated — completion checkpoint`

## 체크포인트

1. `RUN_INTAKE`: run authority, ownership, scope and stop condition.
2. `TOPOLOGY_MODEL`: CPU Set+GLPIEx combination, allocation solver, safeguards and fixtures.
3. `PRIORITY_RECOVERY`: same-handle identity, optional priority and durable recovery.
4. `SERVICE_UI`: settings normalization, service contract, worker live control and Dioxus controls.
5. `VERIFY_REVIEW`: tests, strict Clippy/check, process roundtrip, self-review and canonical docs.
6. `CHECKPOINT_COMMIT`: explicit owned-file staging and local commit.

## 구현 경로와 경계

- `desktop/backend/src/topology.rs`가 topology와 mask allocation의 유일한 권한이다.
- `desktop/backend/src/process.rs`가 Windows process handle 기반 조회·변경의 adapter다.
- `desktop/backend/src/affinity.rs`의 manager만 target process affinity/priority를 조정하고 durable ledger를 소유한다.
- helper control과 service endpoint는 좁은 설정 전달 경로이며 topology나 process mutation 규칙을 재구현하지 않는다.
- `desktop/src/ui.rs`는 status/settings를 표현하며 지원 불가 상태를 숨기지 않는다.

## 검증

- parser/allocation fixtures: hybrid, single/multiple L3, SMT on/off, 2C/4T, 4-core, malformed buffers, mask disjointness, processor groups.
- manager/process tests: defaults, role exclusions, same-handle identity, restore guard, restart recovery.
- commands: backend locked tests, strict Clippy, desktop locked check, Node service-contract checks and Windows child-process priority roundtrip.
- 실제 게임 frame 성능 측정은 `NOT_RUN`으로 분리한다.

## 제약과 rollback

- plan 원본은 수정하지 않는다.
- `NOTICE.md`와 `data/`를 stage하지 않는다.
- 새 위반이나 필수 검증 실패 시 완료/commit을 만들지 않고 handoff에 blocker를 기록한다.
- durable ledger 변경은 이전 entry의 optional field를 허용해 하위 호환한다.
