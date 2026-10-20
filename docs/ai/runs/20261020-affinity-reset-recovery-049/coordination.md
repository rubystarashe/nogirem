# Coordination

- record_id: `COORD-AFFINITY-RESET-RECOVERY-049`
- run_id: `20261020-affinity-reset-recovery-049`
- checkpoint_id: `CP-AFFINITY-RESET-RECOVERY-049`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-20T08:03:00Z`
- target_reference: `uncommitted target based on 4babd317bfef7e325bcf83aca6dd5319888acc8a`
- active_checkpoint: `CP-AFFINITY-RESET-RECOVERY-049`
- DEV: `DONE`
- QA: `COMPLETE_PASS`
- review: `APPROVED, SELF_REVIEW`
- roles: `Coordinator·DEV·DEV-as-QA·SELF_REVIEW·feature/architecture/docs owner = cursor-agent-92251f36`
- runtime_evidence: `reset-all entryCount 30, restoredCount 30이나 이전 프로세스 다수가 0xfc0c 유지; 새 프로세스와 Explorer는 0xffff`
- root_cause: `current mask가 target과 같으면 ledger entry를 만들지 않고 reset이 applied-marker entry를 읽지 않아 crash loop 이전 적용값이 복구 대상에서 누락`
- ownership: `agent owns affinity.rs, affinity_worker.rs, targeted tests and records; user VERSION_HISTORY.md와 기존 dirty files 제외`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — stale CPU affinity reset 누락`
- feature_map: `updated — canonical 42, feature 19`
- architecture_impact: `durable marker recovery and legacy exact-mask migration`
- architecture_contract: `updated — revision 46`
