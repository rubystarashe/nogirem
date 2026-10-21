# Plan

- record_id: `PLAN-WIN10-CACHE-TOPOLOGY`
- run_id: `20261021-win10-cache-topology`
- checkpoint_id: `CP-WIN10-CACHE-TOPOLOGY`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-21T14:04:00Z`
- status: `COMPLETED`
- approach: `RelationCache에서 GroupCount=0이면 같은 union 위치의 legacy GroupMask 한 개를 읽고, 1 이상이면 현재 GroupMasks 배열 개수를 그대로 검증`
- files: `topology.rs, VERSION_HISTORY.md, backend tests, run·feature·architecture·handoff records`
- canonical_path: `service → boost → topology Win32 read adapter`
- checks: `targeted parser test, full backend, desktop check/test, Node contracts, SELF_REVIEW`
- stop_condition: `필수 검사 통과와 local commit`
- package_deploy: `NOT_AUTHORIZED`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST`
- feature_map: `updated — canonical 45, frame 21`
- architecture_impact: `legacy/current CACHE_RELATIONSHIP compatibility`
- architecture_contract: `updated — revision 49`
