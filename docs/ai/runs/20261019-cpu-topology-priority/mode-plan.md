# Plan — CPU allocation modes

- record_id: `PLAN-CPU-MODES-002`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-MODES-002`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-19T06:11:00Z`
- target: `REQ-CPU-MODES-002`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — three CPU allocation presets`
- feature_map: `updated — mode behavior and scenarios`
- architecture_impact: `topology policy remains authoritative; service remains the only CPU setting writer`
- architecture_contract: `updated — mode contract and hybrid overlap exception`

## 구현

1. `topology.rs`에 `Focus`, `Balanced`, `Safe` mode를 두고 게임 집중 자동값을 `min(P-core 수, 균형값+2)`로 계산한다.
2. 비하이브리드는 background P-core 한 개 이상을 남기고, 하이브리드는 모든 P-core 선택 시에만 한 물리 P-core mask를 background와 공유한다.
3. `settings.rs`가 `cpuMode`를 정규화하고 이전 boolean 설정을 migration한다.
4. service가 `optimization:set-cpu-mode`를 통해 저장 후 실행 중 helper를 reset/reconfigure한다.
5. UI는 세 mode button과 계산된 물리 코어 배정을 표시하며 background priority만 별도 고급 옵션으로 유지한다.
6. 기존 `plan.md`는 원 요청의 불변 계획이므로 수정하지 않는다.

## 검증과 중단 조건

- topology fixture: 8P 집중 6P, 균형 4P, 2P+E 공유 1P, 4P 저코어 분할, multi-group fail-safe.
- settings migration, worker/service contract, desktop compile·tests, Node tests와 smoke UI를 실행한다.
- 실제 마비노기 frame·입력 체감은 `NOT_RUN`으로 기록하며 성능 향상을 검증했다고 주장하지 않는다.
- `NOTICE.md`와 `data/`는 stage하지 않는다.
- review_mode: `SELF_REVIEW`; 독립 reviewer는 참여하지 않는다.
