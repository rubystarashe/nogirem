# Handoff — CPU topology and priority

- record_id: `HANDOFF-CPU-TOPOLOGY-PRIORITY-001`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-DEFAULT-BALANCED-012`
- owner: `cursor-agent-92251f36`
- updated_at: `2026-10-19T09:02:06Z`
- status: `COMPLETED`
- DEV: `DONE`
- QA: `COMPLETE_PASS for automated source verification; actual UI/game scenarios NOT_RUN`
- review: `APPROVED`, `SELF_REVIEW`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — 신규 기본 CPU preset을 균형 모드로 변경`
- feature_map: `updated — canonical revision 33, feature revision 12`
- architecture_impact: `CPU 설정 normalization과 helper·UI fallback의 canonical balanced 기본값`
- architecture_contract: `updated — revision 41`

## Resume

- working_directory: `C:\Users\User\Documents\GitHub\nogirem`
- active_checkpoint: `CP-CPU-DEFAULT-BALANCED-012 complete`
- next_authorized_action: 후속 요청 시 실제 UI smoke를 수행한다. Push/deploy remain unauthorized.
- target_manifest: `base 24658824ac86640867209bba40da5faa55223f76 + CP-CPU-DEFAULT-BALANCED-012 owned target; resulting local commit SHA is reported externally`.
- execution_target: `Windows 10.0.26200 x64; AMD Ryzen 7 9800X3D 8C/16T; Rust test/debug target; Node v22.20.0`.
- preserved_out_of_scope: `NOTICE.md`, `data/`, any other baseline dirty content.
- blockers: none.
- completed: 설정이 없는 경우 CPU 기본 preset을 `균형`으로 바꾸고 normalization, topology·boost·worker·UI fallback과 smoke fixture를 일치시켰다. 기존 명시 CPU 설정은 보존하며 channel ping 기본 비활성화는 변경하지 않았다.
- checks: CPU 기본값 targeted 2/2, backend 128 PASS·3 ignored와 parity 2/2, desktop 8/8, locked cargo check PASS.
- limitations: 실제 UI smoke와 actual Mabinogi performance는 `NOT_RUN`.
- findings: no unresolved mandatory findings; `SELF_REVIEW APPROVED`.
- commit/push/deploy: local checkpoint commit 예정; push/deploy/release not performed or authorized.
