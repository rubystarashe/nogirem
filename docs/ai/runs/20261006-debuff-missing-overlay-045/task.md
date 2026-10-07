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

## Follow-up 2026-10-07T09:15:00Z

- request: `앱 시작 자동 추적, 해상도별 체력바·디버프 좌표 저장·재사용, 재인식 지속 retry, 디버프 일렬 slot 좌표 계산 후 slot-only 판독`
- checkpoint_id: `CP-DEBUFF-TRACKING-CACHE-045`
- owner: `Cursor Agent`
- source_base: `51d0ee4a091ffb2d61406a0ee948400a4ae02135`
- source_target: `uncommitted manifest based on source_base; game_overlay.rs, service.rs, service-contract.json, overlay-settings.html, overlay-settings-preload.js, game-overlay.test.mjs와 canonical docs`
- risk: `MEDIUM — foreground 화면 판독 빈도와 durable settings schema를 바꾸지만 기존 원자 writer 하나를 유지하고 화면 pixel은 지속·전송하지 않음`
- roles: `Coordinator=DEV=QA=Reviewer=Feature owner=Architecture owner=Documentation maintainer=Cursor Agent; review_mode=SELF_REVIEW; independent reviewer 없음`
- dev_state: `READY_FOR_QA`
- qa_state: `COMPLETE_PASS for automated source scope`
- review_verdict: `APPROVED for automated source scope`
- delivered: `디버프 알림이 저장된 시작 상태면 monitor가 자동 추적을 시작한다. 전경 정본 게임의 physical client 해상도별로 체력바·디버프 영역을 settings JSON에 저장하고 다음 실행에서 작은 체력바 crop 검증 뒤 재사용한다. stale 좌표는 누락 snapshot과 함께 폐기하고 2초 주기 full detection으로 복귀한다. 재인식 IPC는 즉시 실패하지 않고 찾을 때까지 tracking state를 유지한다. 같은 y의 대상 아이콘 둘 이상에서 허용 범위 내 고정 spacing을 확정하면 phase·y·spacing·count를 저장하고 이후 slot 좌표만 exact RGB 판독한다.`
- canonical_extension_path: `service monitor tick → game_overlay foreground identity·resolution cache → detector 또는 cache validation → full-area discovery 또는 slot-only matcher`
- durable_writer: `game_overlay.rs만 overlay/settings.json의 booleans·alert offset·resolution tracking cache를 원자 갱신`
- security_privacy: `전경 검증된 game HWND만 캡처하고 재인식이 다른 앱 focus를 빼앗지 않음; BGRA·HWND·PID는 지속·로그·전송하지 않음`
- validation: `cargo check --locked --manifest-path desktop/Cargo.toml PASS; cargo test --locked --manifest-path desktop/backend/Cargo.toml game_overlay::tests 18/18 PASS; node --test test/game-overlay.test.mjs 5/5 PASS; diff check PASS`
- manually_exercised: `NOT_RUN — 실제 게임 시작 재로드·해상도 변경·재인식 대기·slot 성능은 사용자 환경 확인 필요`
- app_launch: `npm run app:dev가 최신 source를 빌드하고 nogirem-desktop.exe를 호출했으나 exit 0 뒤 실행 process가 남지 않아 현재 source의 interactive runtime은 확립하지 못함; elevation 또는 기존 single-instance 상태를 이 실행만으로 구분할 수 없음`
- known_gap: `slot layout은 target template 둘 이상이 같은 행에서 허용 spacing을 증명할 때까지 full-area 판독을 유지한다. 실제 게임에서 target 조합이 인접하지 않아 spacing GCD가 허용 범위를 벗어나면 의도적으로 cache를 만들지 않는다.`
- feature_impact: `FEAT-NOGIREM-GAME-OVERLAY — 자동 추적, 해상도별 좌표 cache, 지속 재인식과 slot 한정 판독`
- feature_map: `updated — OVERLAY-002·009·010, durable state, retry·recovery와 evidence 갱신`
- architecture_impact: `foreground periodic capture, resolution-keyed cache single writer, stale validation, slot matcher fast path`
- architecture_contract: `updated — game overlay durable cache·tracking state·slot inference 경계 갱신`
