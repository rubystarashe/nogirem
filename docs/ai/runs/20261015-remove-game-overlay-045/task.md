# 화면 이미지 인식 기반 게임 오버레이 제거

- record_id: `RUN-20261015-REMOVE-GAME-OVERLAY-045`
- run_id: `20261015-remove-game-overlay-045`
- checkpoint_id: `CP-REMOVE-GAME-OVERLAY-045`
- owner: `Cursor Agent`
- created_at: `2026-10-15T05:45:00Z`
- updated_at: `2026-10-15T05:47:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `5f67ca0c41c5a727729b3412458f0c9f886867d3`
- request: `화면 이미지 인식 기능이 공식적으로 금지되었으므로 오버레이 관련 기능을 제거`
- scope_decision: `사용자 확인에 따라 화면 이미지 인식 기반 FEAT-NOGIREM-GAME-OVERLAY만 제거하고 OS TCP 상태를 표시하는 FEAT-NOGIREM-CHANNEL-PING 창은 유지`
- objective: 게임 화면 pixel capture·체력바/디버프 template 판독·좌표 cache·누락 alert의 UI, backend, IPC, auxiliary window와 package 입력을 source와 제품 진입점에서 제거한다.
- exclusions: 채널 핑 상단·전체표, 블랙박스 녹화, 일반 window geometry, 과거 immutable run 기록, 기존 사용자 AppData 파일 삭제, push·package·deploy.
- risk: `MEDIUM — 제품 기능과 IPC·package contract를 삭제하지만 durable migration이나 외부 배포는 없고 별도 채널 핑 기능은 유지`
- authorization: `사용자가 제거를 명시하고 삭제 범위를 화면 이미지 인식 기반 게임 오버레이로 선택함`
- roles: `Coordinator=DEV=QA=Reviewer=Feature owner=Architecture owner=Documentation maintainer=Cursor Agent; review_mode=SELF_REVIEW; independent reviewer 없음`
- dev_state: `DONE`
- qa_state: `COMPLETE_PASS`
- review_verdict: `APPROVED`
- stop_condition: 금지된 runtime·UI·IPC·package 참조가 현재 source에서 제거되고 desktop/backend/Node 회귀 검사와 canonical tombstone·architecture 금지 경계가 완료된다.
- feature_impact: `FEAT-NOGIREM-GAME-OVERLAY — lifecycle REMOVED, 제품 진입점과 구현 전체 삭제`
- feature_map: `updated — REMOVED tombstone, 제거 사유·날짜·범위 기록`
- architecture_impact: `게임 화면 capture·template matcher·overlay settings writer·전용 auxiliary windows 제거; image recognition 재도입 금지`
- architecture_contract: `updated — 화면 이미지 인식 금지 경계와 채널 핑 예외 범위 명시`

## Plan

1. 고급 기능의 게임 오버레이 진입점과 native smoke fixture를 제거한다.
2. `game_overlay.rs`, service state·monitor·IPC, auxiliary window 등록과 preload·HTML을 제거한다.
3. package 입력에서 `debuff.csv`와 추적 condition templates를 제거하고 전용 test를 삭제한다.
4. 현재 source에서 금지된 식별자·asset 의존성이 사라졌는지 검사하고 desktop/backend/Node 회귀 검사를 실행한다.
5. 기능 지도는 identity를 삭제하지 않고 `REMOVED` tombstone으로 전환하며 architecture contract와 handoff를 갱신한다.

## Ownership

- active_writer: `Cursor Agent`
- owned_files: `게임 오버레이 backend/UI/window/IPC/package/test files, canonical feature·architecture docs, handoff, this run`
- out_of_scope_dirty_changes: `사용자 수정 NOTICE.md와 기존 untracked data/active, data/condition.json, 비추적 data/conditions 다수, data/skill.json, native/installer-cleanup은 보존하고 staging에서 제외`

## QA

- execution_target: `uncommitted removal target based on 5f67ca0c41c5a727729b3412458f0c9f886867d3`
- environment: `Windows 10.0.26200 x64, Rust 2024 workspace, Node test runner`
- `QA-REMOVE-001 source reference scan`: `PASS — desktop, scripts, service current source에서 금지된 runtime·UI·IPC·package 식별자 없음`
- `QA-REMOVE-002 mechanical enforcement`: `PASS — test/prohibited-image-recognition.test.mjs가 삭제 파일 부재와 현재 source 금지 의존성을 확인`
- `QA-REMOVE-003 Node regression`: `PASS — 205/205`
- `QA-REMOVE-004 backend regression`: `PASS — 105 passed, 2 declared ignored; integration parity 2/2`
- `QA-REMOVE-005 desktop compile`: `PASS — cargo check --locked --manifest-path desktop/Cargo.toml`
- `QA-REMOVE-006 retained channel overlay`: `PASS — tray_cleanup_preserves_channel_monitoring_windows 1/1`
- `QA-REMOVE-007 IDE diagnostics`: `PASS — edited implementation·test files 오류 없음`
- `QA-REMOVE-008 package/install/manual runtime`: `NOT_RUN — package·배포·설치·게임 실행은 요청 범위가 아니며 제거 source의 자동 검증으로 제한`
- evidence_limitations: 터미널 출력은 transient evidence이며 별도 screenshot·package artifact를 생성하지 않았다. 기존 사용자 AppData의 orphaned overlay 설정 파일은 자동 삭제하지 않는다.

## Review

- review_mode: `SELF_REVIEW`
- reviewer: `Cursor Agent`
- target: `uncommitted removal target based on 5f67ca0c41c5a727729b3412458f0c9f886867d3`
- inspected: UI 진입점, service state·threads·authorization·IPC, window registry, native allowlist, fixture·smoke, package inputs·backend image dependency, deleted HTML·preload·templates, canonical docs, staged scope.
- findings: `필수 finding 없음`
- limitations: `독립 reviewer 없음`
- verdict: `APPROVED`
- feature_impact: `FEAT-NOGIREM-GAME-OVERLAY — REMOVED`
- feature_map: `updated — canonical tombstone과 PASS evidence 반영`
- architecture_impact: `화면 이미지 인식 경로 제거와 mechanical prohibition test 추가`
- architecture_contract: `updated — 금지 경계와 채널 핑 분리 명시`
