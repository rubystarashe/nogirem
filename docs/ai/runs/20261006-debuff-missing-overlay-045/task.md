# 디버프 누락 판독과 이동 가능한 알림 오버레이

- record_id: `RUN-20261006-DEBUFF-MISSING-OVERLAY-045`
- run_id: `20261006-debuff-missing-overlay-045`
- checkpoint_id: `CP-DEBUFF-MISSING-OVERLAY-045`
- owner: `Cursor Agent`
- created_at: `2026-10-06T02:25:00Z`
- updated_at: `2026-10-06T02:25:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `9ce107f97cdbafad0c228777bb49b84d9256d059`
- request: `체력바 바로 위 동일 크기 영역에서 debuff.csv 대상 아이콘을 data/conditions 이미지로 판독하고 없는 디버프를 오버레이로 표시하며 인터페이스 구성 버튼으로 위치를 변경`
- objective: 검출된 보스 체력바를 anchor로 디버프 아이콘을 주기적으로 판독하고 누락 항목만 독립 click-through 창에 표시하며 게임 창 기준 위치를 사용자가 조정·저장한다.
- scope: `debuff.csv`와 현재 16개 condition icon package 입력, foreground screen crop, 순수 template matcher, 누락 snapshot, 독립 alert window, 위치 구성 IPC·저장, 자동 검사와 실제 앱 확인.
- exclusions: 투아림·붕괴·버프·스킬 판독, 게임 메모리·패킷·입력 주입, 원격 이미지, OCR, 자동 전투 조작, package·push·release.
- risk: `HIGH — 지속적으로 foreground 게임 pixel을 읽고 topmost 창을 표시하지만 캡처는 메모리에서 폐기하고 외부 전송·게임 process 접근은 하지 않음`
- authorization: `사용자가 실제 체력바 검출 확인 후 다음 checkpoint 구현을 명시적으로 요청함`
- roles: `Coordinator=DEV=QA=Reviewer=Feature owner=Architecture owner=Documentation maintainer=Cursor Agent; SELF_REVIEW`
- dev_state: `IN_PROGRESS`
- qa_state: `PLANNED`
- review_verdict: `NOT_REVIEWED`
- stop_condition: `아이콘 판독·누락 표시·위치 구성 source와 자동 검사가 통과하고 실제 게임에서 확인하거나 정확한 gap을 기록`
- feature_impact: `FEAT-NOGIREM-GAME-OVERLAY — 실제 디버프 누락 판독·알림과 사용자 배치`
- feature_map: `no_change at intake — 기존 PLANNED 기능의 후속 동작이며 구현 뒤 canonical flow·scenario·evidence를 갱신`
- architecture_impact: `foreground periodic capture, packaged template source, in-memory matcher snapshot, alert position single writer, interactive configuration과 runtime click-through 전환`
- architecture_contract: `no_change at intake — 기존 game overlay capability 경계를 확장하며 구현 뒤 writer·capture·window lifecycle을 갱신`

## Plan

1. 체력바 바로 위의 같은 크기 client-local rectangle을 foreground에서만 캡처한다.
2. `debuff.csv` 순서와 16×16 PNG template을 읽어 원본 1× 불투명 RGB가 전부 같은 경우에만 존재로 판정한다. 확대·유사도·부분 일치·OCR은 사용하지 않는다.
3. 없는 항목의 ID·이름·type·icon data URI만 메모리 snapshot으로 유지한다.
4. 누락 항목이 있을 때 독립 alert window를 표시하고, 설정창의 `인터페이스 구성`으로 위치 조정 mode를 연다.
5. 위치는 체력바 anchor 기준 physical offset으로 `game_overlay.rs`가 기존 settings JSON에 원자 저장한다.
6. IPC/native/package 계약, positive·negative matcher fixture, window lifecycle 검사를 추가한다.

## Ownership

- active_writer: `Cursor Agent`
- owned_files: `game_overlay.rs, service.rs, service_windows.rs, overlay settings/alert HTML·preload, native allowlist, service contract, package script, overlay tests, selected debuff assets, canonical docs`
- out_of_scope_dirty_changes: `installer·portable·antivirus run 변경은 별도 동시 작업으로 보존; debuff.csv와 참조 PNG만 이 checkpoint 범위`

## Follow-up 2026-10-06T04:30:00Z

- feedback: `누락 인터페이스를 물리·마법·보너스·아르카나·이동속도 층으로 분리하고 각 아이콘 아래 CSV 이름 표시; type 3은 물리·마법 양쪽 표시`
- delivered: `debuff.csv의 name을 authoritative 표시 이름으로 읽고 type 1=물리, 2=마법, 3=물리+마법, 4=보너스, 5=이동속도, 6=아르카나로 520×440 alert window에 배치`
- validation: `game overlay Node 5/5, backend game_overlay tests, locked desktop check PASS; debug 앱 재실행`
- review_mode: `SELF_REVIEW`
- review_verdict: `APPROVED for source scope — 독립 reviewer 미참여`
- dev_state: `READY_FOR_QA`
- qa_state: `COMPLETE_PASS for automated scope; 실제 게임 레이어 시각 확인 대기`
- feature_impact: `FEAT-NOGIREM-GAME-OVERLAY — 누락 디버프 분류·중복 표시·이름 가독성`
- feature_map: `updated — type 분류와 alert 표시 flow 반영`
- architecture_impact: `debuff.csv가 ID·type·표시 이름의 package source가 되고 renderer는 backend snapshot을 읽기만 함`
- architecture_contract: `updated — CSV source와 alert renderer read-only 경계 반영`

## Follow-up 2026-10-06T04:35:00Z

- feedback: `구성 완료 뒤 디버프 영역 표시가 남고 투명 창 잔상이 발생함; 완료 버튼을 향후 다른 인터페이스와 공유할 독립 창으로 분리 요청`
- delivered: `overlay-configuration 독립 controller window와 drag·finish 전용 preload 추가; finish가 alert 위치 저장·구성 종료·preview/controller destroy를 조정; 불필요한 preview·alert는 hide 대신 destroy`
- validation: `game overlay Node 6/6, backend game_overlay tests, locked desktop check PASS; debug 앱 재실행`
- review_mode: `SELF_REVIEW`
- review_verdict: `APPROVED for automated source scope`
- qa_state: `COMPLETE_PASS for automated scope; 실제 완료 후 preview 제거와 Windows 잔상 확인 대기`
- feature_impact: `FEAT-NOGIREM-GAME-OVERLAY — 공용 구성 controller와 완료 lifecycle`
- feature_map: `updated — 독립 controller와 destroy lifecycle 반영`
- architecture_impact: `configuration control을 alert renderer에서 분리하고 native auxiliary window 종료를 destroy로 일원화`
- architecture_contract: `updated — controller writer boundary와 transparent window lifecycle 반영`
