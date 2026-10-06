# 게임 오버레이 설정과 디버프 영역 자동인식

- record_id: `RUN-20261006-GAME-OVERLAY-DEBUFF-045`
- run_id: `20261006-game-overlay-debuff-045`
- checkpoint_id: `CP-GAME-OVERLAY-DEBUFF-045`
- owner: `Cursor Agent`
- created_at: `2026-10-06T00:34:00Z`
- updated_at: `2026-10-06T01:42:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `497bb750c7d17ee02b3fe2dc308b873316d51217`
- request: `5종 알림 오버레이 설정창과 보스 체력바 영역 자동인식·오버레이 표시 구현`
- objective: 디버프·투아림·붕괴·버프·스킬 알림 토글을 제공하고, 디버프 자동인식 버튼으로 검증된 마비노기 client의 보스 체력바 외곽을 한 번 검출해 click-through preview로 표시한다.
- scope: 설정 single writer, interactive 설정창, foreground 전환·복원형 GDI client 캡처, 순수 픽셀 검출, 독립 preview window, IPC/native/package 계약, 자동 검사와 canonical 문서.
- exclusions: 실제 디버프 판독·알림, 투아림·붕괴·버프·스킬 감지, 지속 영상 추적, Windows Graphics Capture, package·push·release.
- risk: `HIGH — 검증된 게임 HWND 포커스와 화면 픽셀을 읽고 native topmost overlay를 추가하지만 게임 메모리·패킷·입력 주입·외부 전송은 하지 않음`
- authorization: `사용자가 계획을 검토한 뒤 작업을 명시적으로 승인함`
- roles: `Coordinator=DEV=QA=Reviewer=Feature owner=Architecture owner=Documentation maintainer=Cursor Agent; SELF_REVIEW`
- dev_state: `DONE`
- qa_state: `COMPLETE_PASS for automated scope`
- review_verdict: `APPROVED`
- feature_impact: `FEAT-NOGIREM-GAME-OVERLAY — 5종 알림 설정과 디버프 보스 체력바 영역 자동인식 preview`
- feature_map: `updated 예정 — 신규 identity·흐름·검증·수동 gap 추가`
- architecture_impact: `overlay setting single writer, verified game capture adapter, pure boss-bar detector, interactive settings와 click-through preview window 분리`
- architecture_contract: `updated 예정 — 캡처 trust boundary·포커스 복원·window lifecycle·금지 경계 추가`
- stop_condition: `설정·검출·preview source와 자동 검사가 통과하고 실제 게임 미실행 범위를 명시하거나 정확한 blocker를 기록`

## Confirmed decisions

- 자동인식은 버튼을 누를 때 한 번 수행하고 다시 누르기 전까지 client-local 좌표를 유지한다.
- preview는 클릭 통과 테두리와 작은 `디버프 영역` 라벨이다.
- 캡처 시 기존 foreground를 저장하고 검증된 마비노기 창을 포커싱해 캡처한 뒤 기존 창 포커스를 복원한다.
- 가려진 DirectX 창의 background capture는 검은 화면·stale frame 위험 때문에 이번 checkpoint에서 사용하지 않는다.

## Result

- disposition: `PARTIALLY_COMPLETED — 요청한 source 범위는 구현했고 실제 게임 behavior verification은 NOT_RUN`
- implementation_target: `9aec27b98bee117cc6d14999449a149c9239549c`
- initial_implementation_target: `cb3ee40cee4afda954e2d8ae94a780df39d9fcec`
- delivered: `5종 persistent toggle 설정창, 디버프 one-shot 자동인식, 검증된 게임 foreground GDI client capture와 포커스 복원, client-local 좌표, click-through border·label preview`
- detector: `55±2 하단선, 바로 아래 17±2 선, 20~120px 위 7~12 상단선, endpoint ±2px, RGB channel spread ≤2, 검은 좌우선 ≥95%, aspect ≥5:1`
- provided_image_assessment: `523×40 crop에서 상단 9/10/10, 하단 55/55/55, 좌우 0/0/0과 상단 2px edge gap을 확인해 synthetic BGRA 회귀 fixture에 반영; crop 밖 17 line은 사용자 규칙대로 합성`
- `PASS` — Node 207/207.
- `PASS` — backend 110 passed, 2 declared ignored; legacy·native parity 2/2.
- `PASS` — desktop 8/8, desktop compile.
- `PASS` — detector 5/5, overlay contract 4/4, preload syntax, targeted rustfmt, diff check, IDE diagnostics 0.
- `PASS` — native primary UI smoke에서 고급 기능 row와 `overlay-settings` bridge·5개 toggle·자동인식 문구 렌더링 확인.
- `FAIL after primary PASS` — 전체 smoke 후속 creator-prompt 별도 process가 기존 visual activity 준비 상태를 얻지 못함; 신규 overlay settings 검증이 포함된 primary `ui.json`은 `passed:true`. 제품 source 필수 자동 검사와 직접 관련된 실패는 아님.
- `NOT_RUN` — 실제 마비노기 foreground 전환·복원, GDI pixel capture, 보스 체력바 검출, preview 위치·click-through, 창모드·전체화면·다중 DPI·복수 game window.
- review_mode: `SELF_REVIEW`
- review_scope: `capture trust boundary, PID·start identity, focus restoration, detector tolerance·negative fixtures, setting writer, capability dependency, overlay foreground hiding, IPC authorization, native/package inputs, test evidence`
- review_disclosure: `같은 Cursor Agent가 구현과 검토를 수행했으며 독립 reviewer는 참여하지 않음`
- review_verdict: `APPROVED — unresolved mandatory source finding 0`
- qa_record: `qa.md`
- final_report: `final-report.md`
- feature_gate: `BLOCKED for behavioral completion — canonical FEAT-NOGIREM-GAME-OVERLAY 추가와 자동 source evidence는 완료, 실제 게임 QA NOT_RUN`
- architecture_gate: `PASS — shared geometry 분리, capture·setting single writer와 preview window 경계 명시, 새 예외 없음`
- review_qa_gate: `PASS for automated source scope; BLOCKED for actual-game behavior scope`
- target_authorization_gate: `PASS — local commits only, package·push·release 없음`
- feature_impact: `FEAT-NOGIREM-GAME-OVERLAY — 설정·디버프 영역 자동인식 preview source 구현`
- feature_map: `updated — 신규 PLANNED feature와 source·QA freshness 추가`
- architecture_impact: `foreground capture adapter, pure detector, setting writer, interactive·preview windows`
- architecture_contract: `updated — trust·capture·focus·window·data 경계 추가`
- remaining: `사용자 PC 실제 게임 QA 후 검출 허용오차 조정; 후속 checkpoint에서 실제 디버프 판독·알림`

## Follow-up 2026-10-06T01:29:00Z

- feedback: `자동인식 버튼이 디버프 토글 off 상태에서 비활성, 설정창 scrollbar, close fade-out 누락, SetForegroundWindow 거부`
- disposition: `FIXED in 20ae67c9f20426dbd1f0139d64fb2b408dd817f5`
- changes: `자동인식을 알림 toggle과 분리, 높이 700px·body overflow hidden, 표준 300ms fade path 복원, capture worker를 foreground·target UI thread에 임시 연결해 전환·복원`
- validation: `detector 5/5, overlay/native contract 4/4, locked desktop compile, diff check, IDE diagnostics PASS`
- feature_impact: `FEAT-NOGIREM-GAME-OVERLAY — 설정 UX와 foreground capture 성공 경로 수정`
- feature_map: `updated — toggle 독립 setup flow와 최신 target 반영`
- architecture_impact: `foreground activation adapter의 thread attachment·detach 경계`
- architecture_contract: `updated — source target과 focus activation 경계 갱신`

## Follow-up 2026-10-06T01:42:00Z

- feedback: `foreground 전환 성공 후 실제 보스 화면에서 체력바 영역 미검출`
- diagnosis: `DirectX window DC는 전경이어도 빈 화면·stale surface를 반환할 수 있고 selected bitmap 상태의 GetDIBits read는 유효하지 않을 수 있음`
- disposition: `FIXED in 9aec27b98bee117cc6d14999449a149c9239549c`
- changes: `foreground game client origin·size를 계산해 composited screen DC에서 정확한 영역만 BitBlt하고 bitmap을 memory DC에서 해제한 뒤 GetDIBits 수행`
- validation: `detector 5/5, locked desktop compile, diff check, IDE diagnostics PASS; 수정 관리자 테스트 앱 PID 51960 실행`
- feature_impact: `FEAT-NOGIREM-GAME-OVERLAY — 실제 DirectX 화면 capture adapter 수정`
- feature_map: `updated — 최신 target과 실제 재검증 대기 반영`
- architecture_impact: `foreground-only screen DC crop adapter`
- architecture_contract: `updated — DirectX window DC 금지와 screen DC crop 경계`
