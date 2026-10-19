# Request — CPU topology and priority

- record_id: `REQ-CPU-TOPOLOGY-PRIORITY-001`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-TOPOLOGY-PRIORITY-001`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-19T05:09:00Z`
- source_target: `main at 1502358cc77df6a35e3515cf5ec98ddd2f0737ab with an uncommitted owned manifest to be produced`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — CPU topology, affinity allocation, low-core protection, optional game binding and background priority`
- feature_map: `updated — required after implementation and verification`
- architecture_impact: `topology authority, process-handle mutation adapter, durable affinity ledger and service IPC`
- architecture_contract: `updated — required after implementation and verification`

## 요청과 목표

Windows CPU Set과 `GetLogicalProcessorInformationEx`를 결합해 물리 코어·SMT·P/E 분류·L3 locality를 판별한다. 선택한 P-core만 게임에 배정하고 백그라운드와 겹치지 않게 하며, 물리 코어 4개 이하는 affinity 변경을 하지 않는다. 게임 hard affinity와 백그라운드 priority 하향은 고급 옵션으로 제공하고 priority 옵션은 기본 비활성으로 둔다.

## 범위와 제외

- 포함: Rust/Dioxus 정본 topology, affinity helper, process adapter, 설정 IPC와 고급 UI, durable recovery, 자동·실프로세스 검증, canonical records.
- 포함: 복수 processor group을 감지하고 현재 단일-mask 적용을 fail-safe로 비활성화.
- 제외: 외부 process의 모든 thread를 순회하는 불완전한 multi-group 이동, 배포·push·release, 실제 게임 성능 향상 주장.
- 보존: 기존 사용자 수정 `NOTICE.md`, 비추적 `data/`, 그 밖의 비소유 변경.

## 수용 기준

1. 게임·백그라운드 mask가 겹치지 않고 요청한 물리 P-core 수가 실제 게임 mask에 반영된다.
2. 같은 L3 domain을 우선하되 cache 크기를 성능 등급으로 사용하지 않는다.
3. 4개 이하 물리 코어와 복수 processor group은 affinity 변경 없이 명시적인 사유를 반환한다.
4. 기본 모드는 백그라운드만 이동하고, game binding을 켠 경우에만 게임 affinity를 변경한다.
5. background priority 옵션은 기본 false이며 일반 background role에만 `BELOW_NORMAL_PRIORITY_CLASS`를 적용한다.
6. 원본 affinity와 priority를 process identity와 함께 기록하고 앱 적용값이 유지된 경우에만 복원한다.
7. 필수 자동 검사와 self-review 또는 독립 review 결과가 exact target에 기록된다.

## 위험 분류

`high`: 다른 프로세스의 실행 위치·priority와 durable 복구를 변경하고 Windows topology/API 계약 및 UI 설정을 함께 바꾼다. 부분 적용, PID 재사용, 외부 writer 충돌, 저코어 성능 저하가 제품 동작에 직접 영향을 준다.
