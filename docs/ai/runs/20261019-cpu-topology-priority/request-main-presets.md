# Request correction — main-screen CPU presets

- record_id: `REQ-CPU-MAIN-PRESETS-004`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-MAIN-PRESETS-004`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-19T06:37:00Z`
- supersedes: `REQ-CPU-MODE-SPLIT-003` for preset composition and placement
- source_target: `main at 17dee9063db8c9926d70a1f9eab5a4955b7c47d8; NOTICE.md and data/ preserved`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — main-screen focus/balanced/safe presets plus advanced manual count`
- feature_map: `updated — revision 5`
- architecture_impact: `preset orchestration in service/UI; topology and affinity ownership unchanged`
- architecture_contract: `updated — revision 38`

## 최종 요청

- 메인 화면 왼쪽의 Vulkan 관리 버튼 바로 아래에 `게임 집중`, `균형`, `안전` 세 CPU preset을 둔다.
- `게임 집중`: 공격적 자동 P-core 수와 game hard affinity. Ryzen 7 9800X3D는 6P.
- `균형`: 기존 자동 P-core 수와 game hard affinity. Ryzen 7 9800X3D는 4P.
- `안전`: 균형 자동 P-core 수로 background를 이동하지만 game affinity는 고정하지 않는다. Ryzen 7 9800X3D 표시 count는 4P.
- 고급 기능의 수동 core count 선택은 유지한다. 수동 선택은 현재 mode를 유지하며 preset 자동 count를 override한다.
- 세 preset 중 하나를 다시 누르면 custom count를 제거하고 해당 preset 자동값으로 돌아간다.

## 수용 기준

1. main preset은 Vulkan 관리 control 아래에 보이고 세 label과 선택 상태를 제공한다.
2. 8P에서 focus 6P, balanced/safe 4P다.
3. safe만 game affinity를 적용하지 않는다.
4. 고급 기능에서 5P 등 수동값을 선택해도 현재 mode가 유지된다.
5. preset 재선택은 custom count를 preset 자동값으로 초기화한다.
6. hybrid 공유, low-core, processor-group fail-safe, priority와 ledger 규칙은 유지된다.

## 위험 분류

`medium`: main interaction과 process affinity preset을 변경하지만 single writer, durable restore와 외부 contract 경계는 유지된다. 실제 마비노기 성능 비교는 별도 QA다.
