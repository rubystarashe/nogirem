# Request follow-up — CPU allocation modes

- record_id: `REQ-CPU-MODES-002`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-MODES-002`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-19T06:10:00Z`
- source_target: `main at 17dee9063db8c9926d70a1f9eab5a4955b7c47d8 with NOTICE.md and data/ preserved out of scope`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — 게임 집중·균형·안전 모드와 mode별 P-core 배정`
- feature_map: `updated — follow-up 완료 시 revision 3`
- architecture_impact: `topology allocation policy, normalized CPU mode setting, worker control and UI contract`
- architecture_contract: `updated — follow-up 완료 시 revision 36`

## 사용자 후속 결정

- CPU 모드는 `게임 집중`, `균형`, `안전 모드` 세 가지로 구성한다.
- `게임 집중`은 기존 균형값보다 P-core를 최대 2개 더 사용하며 Ryzen 7 9800X3D에서는 6개를 사용한다.
- `균형`은 기존 자동값을 유지하며 Ryzen 7 9800X3D에서는 4개를 사용한다.
- `안전 모드`는 게임 affinity를 고정하지 않고 Windows scheduler에 맡기되 background 이동은 유지한다.
- 하이브리드 CPU에서 게임 대상은 P-core로 제한한다. 모든 P-core가 게임에 선택되어 background가 E-core만 사용하게 되는 경우 P-core 한 개를 background와 공유한다.
- 물리 코어 4개 이하라는 이유만으로 분할을 끄지 않는다. 단일 P-core 비하이브리드처럼 유효한 두 역할 mask를 만들 수 없는 경우만 비활성화한다.

## 수용 기준

1. 8P/16T fixture의 게임 집중 자동값은 6P, 균형 자동값은 4P다.
2. 2P+E hybrid 게임 집중은 게임 2P와 background E+공유 1P를 만든다.
3. 모드 변경은 저장 설정, 실행 중 helper, service status와 UI에 같은 값으로 반영된다.
4. 이전 `bindGameAffinity` 설정은 `true→balanced`, `false→safe`로 읽고 새 설정이 없는 사용자는 `focus`를 사용한다.
5. background priority 옵션은 세 모드와 독립적으로 유지된다.
6. 기존 원본-state ledger와 guarded restore 경계는 변경하지 않는다.

## 위험 분류

`medium`: affinity 대상 mask와 기본 모드를 바꾸므로 실제 게임·background 성능에 영향이 있다. durable data schema는 단순 enum migration이고 process restore writer·보안 경계·외부 integration은 변경하지 않는다.
