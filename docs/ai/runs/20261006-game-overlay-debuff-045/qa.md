# 게임 오버레이 디버프 자동인식 QA

- record_id: `QA-20261006-GAME-OVERLAY-DEBUFF-045`
- run_id: `20261006-game-overlay-debuff-045`
- checkpoint_id: `CP-GAME-OVERLAY-DEBUFF-045`
- owner: `Cursor Agent acting as QA`
- created_at: `2026-10-06T01:05:00Z`
- source_target: `9aec27b98bee117cc6d14999449a149c9239549c`
- execution_target: `Windows 10.0.26200 x64 source tests, synthetic BGRA fixtures, Dioxus native smoke fixture`
- qa_state: `COMPLETE_PASS for automated scope`
- feature_impact: `FEAT-NOGIREM-GAME-OVERLAY — 설정·자동인식·preview source 검증`
- feature_map: `updated — scenarios와 자동 evidence·수동 gaps 반영`
- architecture_impact: `capture trust·detector·setting writer·window/IPC/package 경계 검사`
- architecture_contract: `updated — 신규 경계와 실제 QA gap 반영`

## Executed

- `PASS` — `npm test`: 207/207.
- `PASS` — backend: 110 passed, 2 declared ignored; parity 2/2.
- `PASS` — desktop: 8/8 and locked compile.
- `PASS` — detector 5/5: height 22·40·61·100, line endpoint ±2, asymmetric 9/10/10 top line, 95% vertical edge, missing double/side negatives, setting roundtrip.
- `PASS` — overlay contract 4/4 and native preload contract.
- `PASS` — primary native UI smoke: advanced row, settings bridge, five toggles, auto-detection button; artifact `C:\Users\User\AppData\Local\Temp\nogirem-dioxus-smoke-cRfIHL\ui.json`.
- `PASS` — targeted rustfmt, diff check, IDE diagnostics.
- `PASS` — follow-up: toggle-independent detection, 700px no-scroll window contract, fade path, foreground UI-thread attachment compile and detector 5/5.
- `PASS` — follow-up: composited screen DC client crop과 bitmap deselect compile, detector 5/5, diff check, IDE diagnostics.
- `PASS` — follow-up: 실제 렌더링 허용오차·상단/세로선 fallback detector 9/9, backend 전체, Clippy `-D warnings`, diff check, IDE diagnostics.
- `PASS` — `OVERLAY-003`: 사용자가 3440×1440 실제 마비노기 보스 화면에서 체력바 영역 인식 성공을 확인함.
- `PASS` — `OVERLAY-003 retry`: 기존 preview를 제거하고 monitor를 직렬화한 재인식 source path를 검증함.
- `FAIL after primary PASS` — full smoke's later isolated creator-prompt process reported `visual=false`; primary overlay UI report remained `passed:true`.

## Not run

- 실제 foreground 복원 결과의 별도 영상 증거.
- 실제 보스 체력바 negative 검출.
- preview 위치·click-through·Alt+Tab hide.
- 100~200% DPI, 보조 모니터, 창모드·무테두리·독점 전체화면.
- 일반·테스트 client 동시 실행의 ambiguity 오류.

## Assessment

- automated source acceptance: `PASS`
- actual game behavioral acceptance: `PARTIAL_PASS — 실제 3440×1440 positive 검출은 사용자 확인, negative·DPI·다중 모니터는 NOT_RUN`
- evidence limitation: `제공 crop은 하단 17 line이 잘려 있어 상단·55 line·좌우선 값만 실제 pixel로 확인했고 17 line은 사용자 규칙대로 synthetic fixture에서 검증`
