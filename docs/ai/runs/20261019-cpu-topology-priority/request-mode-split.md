# Request correction — manual count and affinity mode

- record_id: `REQ-CPU-MODE-SPLIT-003`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-MODE-SPLIT-003`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-19T06:29:00Z`
- supersedes: `REQ-CPU-MODES-002`
- source_target: `main at 17dee9063db8c9926d70a1f9eab5a4955b7c47d8; NOTICE.md and data/ preserved`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — 수동 core count와 affinity mode를 독립 설정`
- feature_map: `updated — revision 4`
- architecture_impact: `topology count selection and affinity binding responsibility separated`
- architecture_contract: `updated — revision 37`

## 정정된 요청

CPU 옵션을 다음 두 영역으로 나눈다.

1. `게임 CPU 코어 할당 개수`: 사용자가 물리 P-core 개수를 직접 선택한다. 기존 자동값은 초기값일 뿐 mode가 개수를 자동 변경하지 않는다.
2. `CPU 할당 모드`: `게임 집중`은 게임을 선택한 P-core mask에 고정하고, `안전 모드`는 게임 affinity를 고정하지 않고 Windows scheduler에 맡긴다.

mode를 바꿔도 사용자가 선택한 core count는 보존한다. 따라서 Ryzen 7 9800X3D 사용자는 초기 4P에서 6P로 직접 바꾼 뒤 게임 집중 또는 안전 모드를 독립적으로 선택할 수 있다.

## 수용 기준

- 8P fixture에서 수동 6P가 정확히 game mask에 반영된다.
- focus↔safe 전환 후에도 `gameCoreCount=6`이 유지된다.
- UI가 수동 count와 두 mode를 별도 제목과 control로 표시한다.
- hybrid all-P 선택은 background에 한 물리 P-core를 공유한다.
- `EfficiencyClass`는 P/E 소속, `SchedulingClass`는 같은 P-core 후보 우선순위로만 사용한다.
- 기존 ledger·priority option·multi-group fail-safe는 유지한다.

## 위험 분류

`medium`: process affinity mask와 기본 focus mode에 영향을 주지만 durable ledger writer, 외부 contract와 보안 경계는 유지된다. 실제 게임 성능은 자동 검증만으로 확정하지 않는다.
