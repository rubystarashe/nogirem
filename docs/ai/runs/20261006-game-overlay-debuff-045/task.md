# 게임 오버레이 설정과 디버프 영역 자동인식

- record_id: `RUN-20261006-GAME-OVERLAY-DEBUFF-045`
- run_id: `20261006-game-overlay-debuff-045`
- checkpoint_id: `CP-GAME-OVERLAY-DEBUFF-045`
- owner: `Cursor Agent`
- created_at: `2026-10-06T00:34:00Z`
- updated_at: `2026-10-06T00:34:00Z`
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
- dev_state: `IN_PROGRESS`
- qa_state: `PLANNED`
- review_verdict: `NOT_REVIEWED`
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
