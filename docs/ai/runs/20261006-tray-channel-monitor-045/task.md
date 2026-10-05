# 트레이 상태 채널 모니터링 유지

- record_id: `RUN-20261006-TRAY-CHANNEL-MONITOR-045`
- run_id: `20261006-tray-channel-monitor-045`
- checkpoint_id: `CP-TRAY-CHANNEL-MONITOR-045`
- owner: `Cursor Agent`
- created_at: `2026-10-05T23:49:00Z`
- updated_at: `2026-10-05T23:54:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `543064863d3316478763395929565f1fc78db27a`
- request: `부스터가 트레이로 들어가면 채널별 점수, 중앙상단 핑 표시 기능이 동작하지 않음`
- objective: 메인 창을 트레이로 숨겨도 활성화된 채널 측정, Windows 키 전체표, 게임 전경 상단 compact 표시를 유지한다.
- scope: 트레이 보조 창 정리 정책, 회귀 단위 테스트, 관련 자동 검사와 handoff.
- exclusions: 측정 공식·TCP EStats·foreground 판정 변경, 버전 증가, package·push·release 재배포.
- risk: `MEDIUM — 트레이 전환 중 native 보조 창 수명주기를 바꾸지만 채널 창 두 종류로 보존 범위를 제한하고 데이터·계약·보안 경계는 변경하지 않음`
- roles: `Coordinator=DEV=QA=Reviewer=Feature owner=Architecture owner=Documentation maintainer=Cursor Agent; SELF_REVIEW`
- dev_state: `DONE`
- qa_state: `COMPLETE_PASS`
- review_verdict: `APPROVED`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — 트레이 상태에서도 채널 점수 측정과 compact·expanded 표시 유지`
- feature_map: `updated — PING-012 tray 상태 유지 흐름과 최신 source·검증 target을 반영`
- architecture_impact: `native tray auxiliary-window cleanup에서 채널 모니터링 창 수명주기를 main window visibility와 분리`
- architecture_contract: `updated — 두 channel monitor WebView를 main tray visibility와 분리하는 기존 경계를 명시`
- stop_condition: `트레이 최소화가 일반 보조 창은 닫되 channel-ping-overlay·channel-ping-live를 보존한다는 자동 검사가 통과하고 회귀 위험·미실행 수동 QA가 기록됨`

## Diagnosis

- `Windows::minimize()`와 tray-hidden 상태의 `focus_main()`이 `destroy_aux()`를 호출한다.
- `destroy_aux()`가 notice를 제외한 모든 보조 창을 제거해 두 channel ping 창도 파괴한다.
- overlay monitor의 `InputMonitor`는 계속 살아 있으므로 `prepare_channel_ping()` 조건으로 돌아가지 않아 두 창이 재생성되지 않는다.
- channel scheduler는 별도 thread라 측정 설정은 유지되지만 사용자가 보는 전체표와 상단 compact가 모두 사라진다.

## Plan

1. tray auxiliary cleanup이 두 channel ping 창을 보존하도록 종류별 정책을 명시한다.
2. notice·channel ping 보존과 일반 보조 창 제거를 단위 테스트한다.
3. channel ping·desktop 자동 검사와 lint·diff check를 실행한다.
4. SELF_REVIEW, handoff, final disposition을 기록하고 로컬 checkpoint commit을 만든다.

## Result

- disposition: `COMPLETED`
- implementation_target: `66d56b49544219fa7c8f38d5aea77a87f78bb11c`
- implemented: `destroy_aux()`의 tray cleanup이 `channel-ping-overlay`와 `channel-ping-live`를 보존하고 character guide·blackbox manager 같은 일반 보조 창은 계속 제거함.
- `PASS` — tray window policy regression 1/1.
- `PASS` — backend 105 passed, 2 declared ignored; legacy·native parity 2/2.
- `PASS` — overlay interface 3/3.
- `PASS` — targeted Rust formatting, `git diff --check`, IDE diagnostics 0.
- `NOT_RUN` — 실제 tray 최소화 중 게임 전경 compact, Windows 키 expanded, tray 복원, 혼합 DPI.
- failed_check: `desktop/backend/Cargo.toml 직접 --locked 실행은 기존 nested Cargo.lock 갱신 요구로 test 시작 전에 실패; 정본 desktop workspace lock 명령으로 동일 targeted test와 전체 backend를 재실행해 통과`
- review_mode: `SELF_REVIEW`
- review_scope: `tray minimize·restore call sites, destroy_aux 종류 filter, channel monitor thread 재준비 조건, 일반 보조 창 정리 회귀, canonical feature·architecture records`
- review_disclosure: `같은 Cursor Agent가 구현과 검토를 수행했으며 독립 reviewer는 참여하지 않음`
- review_verdict: `APPROVED — unresolved mandatory finding 0`
- feature_gate: `PASS for automated source scope — PING-012와 freshness 갱신; 실제 tray·game behavior는 NOT_RUN`
- architecture_gate: `PASS — main visibility와 channel monitor windows의 독립 수명주기 복구, 새 dependency·writer·exception 없음`
- review_qa_gate: `PASS for mandatory automated scope`
- target_authorization_gate: `PASS — 로컬 구현·검증·commit만 수행; package·push·release 없음`
- package_status: `NOT_RUN — 사용자 요청 범위 아님`
- push_status: `NOT_RUN — 사용자 요청 범위 아님`
- release_status: `NOT_RUN — 공개 v0.4.5 package에는 이 후속 fix가 포함되지 않음`
- remaining: `사용자 환경에서 실제 tray·게임·Windows 키 확인 후 필요하면 별도 버전 package·배포`
