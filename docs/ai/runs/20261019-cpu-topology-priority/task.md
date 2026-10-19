# CPU 기본 균형 모드 체크포인트

- record_id: `TASK-CPU-DEFAULT-BALANCED-001`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-DEFAULT-BALANCED-012`
- owner: `cursor-agent-92251f36`
- roles: `Coordinator=DEV=QA=Reviewer=Feature owner=Architecture owner=Documentation maintainer=cursor-agent-92251f36`
- updated_at: `2026-10-19T09:02:06Z`
- status: `COMPLETED`
- DEV: `DONE`
- QA: `COMPLETE_PASS`
- review_mode: `SELF_REVIEW`
- review_verdict: `APPROVED`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — 저장 CPU 설정이 없는 신규 기본값을 균형 모드로 변경`
- feature_map: `updated — canonical map revision 33, feature revision 12`
- architecture_impact: `CPU 설정 normalization과 helper·UI 방어 fallback의 canonical balanced 기본값`
- architecture_contract: `updated — revision 41`

## 요청과 범위

- original_request: CPU 기본값을 균형 모드로 바꾼다.
- superseding_clarification: 실시간 게임 서버 상태 확인은 기본 비활성화를 유지한다.
- acceptance: 설정 파일이 없으면 `balanced`, game affinity ON, background priority 하향 OFF를 사용한다. 명시 저장된 focus·balanced·safe와 수동 설정은 보존한다. channel ping의 기본 `false`는 변경하지 않는다.
- exclusions: 기존 사용자 설정 강제 migration, channel ping 기본값 변경, package, push, deploy.
- risk: `LOW — 기본 선택만 변경하며 durable schema·writer·외부 API·보안 경계는 바뀌지 않는다. 다만 CPU 배치 동작에 영향을 주므로 backend 전체 회귀와 desktop compile/test를 수행했다.`

## 계획과 구현

- 설정 정규화, topology 기본값, boost/helper/UI fallback을 `balanced`로 통일한다.
- smoke fixture와 초기 dropdown assertion을 균형 4/8P·priority OFF로 맞춘다.
- 설정 부재와 명시 설정 보존 회귀를 검증한다.
- canonical feature map과 architecture contract의 기본값 계약을 갱신한다.

## 대상과 검증

- source_base: `24658824ac86640867209bba40da5faa55223f76`
- source_target: `위 base + CP-CPU-DEFAULT-BALANCED-012 owned working-tree target`
- changed_scope: `desktop/backend/src/{settings.rs,topology.rs,boost.rs,affinity_worker.rs,service_fixture.rs}`, `desktop/src/{ui.rs,smoke.rs}`, `VERSION_HISTORY.md`, canonical wiki 2개, run/handoff 기록
- out_of_scope_preserved: 사용자 수정 `NOTICE.md`, 비추적 `data/`
- `PASS` — CPU 기본값 targeted tests 2/2
- `PASS` — backend 128 passed, 3 declared ignored, parity 2/2
- `PASS` — desktop 8/8
- `PASS` — `cargo check --manifest-path desktop/Cargo.toml --locked`
- `NOT_RUN` — 실제 UI smoke와 마비노기 성능 비교
- formatting: repository-wide `cargo fmt --check`는 기존 비포맷 source 때문에 실패했으며 이번 변경 줄의 신규 형식 문제는 수정했다.

## 자체 검토와 최종 판정

- inspected: 설정 부재 경로, legacy 명시값 migration, runtime/helper/UI fallback, fixture와 smoke 기대값, channel ping 기본값 비변경, canonical 문서.
- findings: mandatory finding 없음.
- limitations: 독립 reviewer 미참여. 실제 앱 UI와 게임은 실행하지 않음.
- feature_gate: `PASS — BOOST-014와 exact default behavior 기록`
- architecture_gate: `PASS — single writer와 schema는 유지하고 canonical fallback만 갱신`
- review_qa_gate: `PASS for automated source target; manual UI/game scenarios are non-mandatory for this low-risk default change`
- target_authorization_gate: `PASS for local commit; push/deploy not authorized`
- commit_push_deploy: local checkpoint commit 예정, push/deploy 미수행.
- remaining: 실제 UI smoke는 후속 요청 시 수행.
