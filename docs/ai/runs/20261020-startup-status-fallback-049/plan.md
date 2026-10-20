# Plan

- record_id: `PLAN-STARTUP-STATUS-049`
- run_id: `20261020-startup-status-fallback-049`
- checkpoint_id: `CP-STARTUP-STATUS-049`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-20T10:05:00Z`
- status: `COMPLETED`
- implementation: `startup_state를 150ms 간격 최대 3회 조회하고 read-only 조회 실패만 unknown result로 변환한다`
- UI: `unknown result는 전역 오류 대신 Windows 시작 시 트레이 실행 카드에 확인 불가·비활성으로 표시한다`
- mutation_rule: `사용자가 설정을 변경한 뒤 조회·검증이 실패하면 기존 오류를 유지한다`
- tests: `Rust retry/fallback unit, Node source contract, full backend·desktop·Node release gates`
- package: `clean committed source에서 signed 0.4.9 설치형·포터블과 canonical 9 assets 생성`
- deployment: `origin/main fast-forward, 기존 annotated v0.4.9 tag를 새 exact source로 교체하고 Rust 6 assets를 feed-safe order로 대치`
- rollback: `공개 검증 실패 시 새 manifests를 공개 상태로 두지 않고 이전 verified asset set으로 복구`
- stop_condition: `public 9/9 hash·Ed25519·aliases·Electron routing과 final records push`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE`
- feature_map: `updated — revision 14`
- architecture_impact: `startup read adapter failure containment`
- architecture_contract: `updated — revision 48`
