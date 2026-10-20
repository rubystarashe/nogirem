# Plan — affinity helper single writer 0.4.9

- record_id: `PLAN-AFFINITY-SINGLE-WRITER-049`
- run_id: `20261020-affinity-single-writer-049`
- checkpoint_id: `CP-AFFINITY-SINGLE-WRITER-049`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-20T04:10:00Z`
- target_reference: `640de3b4a3fd7ca646f4f8310a2ce7f71e0a653e`
- status: `COMPLETED`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — affinity helper 단일 writer와 시작 오류 진단 개선`
- feature_map: `updated — canonical revision 40, feature revision 17`
- architecture_impact: `helper lock ownership, startup status, affinity original-state ledger single-writer 경계`
- architecture_contract: `updated — revision 44`

## Checkpoint

1. `COMPLETED` — lock record에 exact process identity를 기록하고 살아 있는 owner lock 탈취를 금지한다.
2. `COMPLETED` — drop은 현재 lock record가 자신의 exact identity일 때만 삭제한다.
3. `COMPLETED` — launch가 status를 선삭제하지 않고 fresh live worker만 성공으로 인정하며 child stderr를 전달한다.
4. `COMPLETED` — regression tests와 0.4.9 version/history를 반영한다.
5. `COMPLETED` — locked source checks, signed package, payload·signature·10MB gate를 검증한다.
6. `COMPLETED` — canonical records·QA·handoff를 완료하고 owned files만 commit한다.

## 구현·검증 경계

- canonical_extension_path: `boost.rs worker coordinator → storage.rs HelperLock → affinity_worker.rs authoritative writer`
- durable_writer: affinity helper 하나만 `status.json`, `runtime-state.json`, `applied-marker.json`을 쓴다.
- lock_contract: 살아 있는 exact process identity의 lock은 status 파일 부재·staleness만으로 삭제하지 않는다.
- startup_contract: coordinator는 기존 status를 선삭제하지 않고 새 worker가 기록한 fresh live status 또는 child stderr를 기준으로 결과를 판정한다.
- rollback_constraint: package·release 산출물은 source에 포함하지 않으며 외부 배포하지 않는다.
- test_plan: HelperLock active/stale/drop ownership tests, backend full locked tests, desktop tests/check, Node suite, native integration, signed package contracts와 manifest 검증.
- stop_condition: mandatory checks PASS, unresolved must-fix 없음, exact package target 기록, commit 완료.
