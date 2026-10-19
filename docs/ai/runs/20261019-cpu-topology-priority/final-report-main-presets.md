# Final report — main CPU presets

- record_id: `FINAL-CPU-MAIN-PRESETS-004`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-MAIN-PRESETS-004`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-19T06:51:00Z`
- disposition: `COMPLETED`
- source_target: `base 17dee9063db8c9926d70a1f9eab5a4955b7c47d8 + EVID-CPU-MAIN-PRESETS-SOURCE-004`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — main focus/balanced/safe presets and advanced manual count`
- feature_map: `updated — revision 5`
- architecture_impact: `preset defaults, topology classification, settings/worker IPC and main UI placement`
- architecture_contract: `updated — revision 38`

## Delivered

- 메인 화면 왼쪽 Vulkan 관리 control 아래에 `게임 집중`, `균형`, `안전` preset을 배치했다.
- Ryzen 7 9800X3D/8P 기준 focus는 6P hard affinity, balanced는 4P hard affinity, safe는 4P background 분할과 game 미고정이다.
- 고급 기능에서 P-core 수를 직접 선택할 수 있고 현재 mode를 유지한다. main preset 재선택은 custom count를 제거해 자동값을 복원한다.
- P/E membership은 `EfficiencyClass`, 같은 P-core 후보 우선순위는 `SchedulingClass`, physical/SMT/L3/group authority는 GLPIEx를 사용한다.
- hybrid에서 모든 P-core를 game에 선택하면 background가 E-core-only가 되지 않도록 한 물리 P-core를 공유한다. 저코어는 유효 mask가 가능한 한 분할한다.
- same-handle mutation, durable ledger, guarded restore와 background-only priority opt-in은 유지했다.

## Verification and review

- automated: backend 127 passed·3 ignored plus parity 2/2; desktop 8/8; Node 213/213; desktop locked check와 IDE diagnostics PASS.
- manually exercised by desktop smoke: main preset 위치·label, focus 6P, balanced/safe 4P, advanced manual 5P mode 보존과 settings roundtrip.
- review_mode: `SELF_REVIEW`; `REV-CPU-MAIN-PRESETS-004` APPROVED, `ISSUE-PRESET-001`~`004` resolved.
- `NOT_RUN`: 실제 마비노기 frame time·입력 latency·background 체감, packaged helper forced-exit recovery.
- repository-wide rustfmt check: 기존 compact-format source baseline 때문에 nonzero; 변경 diff의 `git diff --check`와 IDE diagnostics는 통과했다.

## Gates and publication

- feature gate: `PASS` for source and automated UI/service scope; actual game performance evidence gap disclosed.
- architecture gate: `PASS`; topology/settings/process writer 경계와 multi-group fail-safe 유지.
- review/QA gate: `PASS` with disclosed SELF_REVIEW; no unresolved mandatory findings.
- target/authorization gate: `PASS` after explicit owned-file staging; `NOTICE.md` and `data/` excluded.
- commit: authorized local checkpoint commit 예정이며 최종 SHA는 commit 밖에서 보고한다.
- push/deploy/release: 수행하지 않았고 권한 없음.
