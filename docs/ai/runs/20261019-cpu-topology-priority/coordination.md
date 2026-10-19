# Coordination — CPU topology and priority

- record_id: `COORD-CPU-TOPOLOGY-PRIORITY-001`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-TOPOLOGY-PRIORITY-001`
- owner: `cursor-agent-92251f36`
- updated_at: `2026-10-19T05:35:00Z`
- exact_target: `EVID-CPU-TOPOLOGY-SOURCE-001; patch SHA-256 f0f3b3d3c3eb1a869ea39f19cc59b569c2cff913865fd98604b128dfe0c30c18`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — CPU allocation and background priority behavior`
- feature_map: `updated — revision 31`
- architecture_impact: `topology/process/ledger/service boundaries`
- architecture_contract: `updated — revision 35`

## 역할과 소유권

- Coordinator, DEV, QA, Reviewer, Feature owner, Architecture owner, Documentation maintainer: `cursor-agent-92251f36`
- review_mode: `SELF_REVIEW`
- 독립 reviewer: `none`
- 코드·run record 활성 writer: `cursor-agent-92251f36`
- 사용자 소유로 보존: `NOTICE.md`, `data/`, 그 밖의 baseline dirty files.

## 현재 상태

- active_checkpoint: `CHECKPOINT_COMMIT`
- DEV: `IN_PROGRESS`
- QA: `COMPLETE_PASS`
- review: `APPROVED`, `SELF_REVIEW`
- stop_condition: 구현, mandatory checks, SELF_REVIEW, canonical docs, exact-target manifest와 local checkpoint commit 완료.

## 결정과 경계

- multi-group CPU에서 단일 `u64` process affinity mask를 적용하지 않는다.
- background만 기본 이동하고 game binding은 명시적 opt-in이다.
- priority 변경은 일반 background role에만 opt-in으로 적용한다.
- ledger restore는 동일 process identity이며 현재 값이 앱 적용값과 일치할 때만 수행한다.
- 외부 게시·push·배포·release는 권한 범위 밖이다.

## 동시 작업과 미해결 항목

- ownership overlap: `NOTICE.md`와 `data/` 제외로 해소.
- exact source target, QA evidence and self-review: 확정.
- local checkpoint commit: pending.
- cross-repository dependency: 없음.
