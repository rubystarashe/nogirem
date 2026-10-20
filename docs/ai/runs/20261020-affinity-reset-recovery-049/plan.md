# Plan

- record_id: `PLAN-AFFINITY-RESET-RECOVERY-049`
- run_id: `20261020-affinity-reset-recovery-049`
- checkpoint_id: `CP-AFFINITY-RESET-RECOVERY-049`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-20T08:03:00Z`
- target_reference: `uncommitted target based on 4babd317bfef7e325bcf83aca6dd5319888acc8a`
- status: `COMPLETED`
- implementation: `reset에서 current ledger와 applied marker entry를 합쳐 복구하고, 알려진 entry 밖에서 현재 topology가 생성한 exact app mask를 가진 eligible process만 full system mask 복구 entry로 승격한다`
- safety: `known identity는 legacy fallback에서 제외하고 exact applied masks 이외 값, excluded·Windows·다른 session process는 변경하지 않는다`
- durable_writer: `affinity helper 단일 writer가 synthetic legacy recovery entry를 marker에 먼저 기록한 뒤 복구한다`
- tests: `pure stale-entry selection, backend locked, desktop locked, Node full, packaged isolated reset recovery`
- package: `npm run package:win signed 0.4.9 installer·portable`
- stop_condition: `필수 QA PASS, SELF_REVIEW APPROVED, exact package target과 evidence, local commits`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — 중단 복구 완결성`
- feature_map: `updated — canonical 42, feature 19`
- architecture_impact: `marker-ledger union과 exact-mask migration recovery`
- architecture_contract: `updated — revision 46`
