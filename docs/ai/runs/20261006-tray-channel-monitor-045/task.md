# 트레이 상태 채널 모니터링 유지

- record_id: `RUN-20261006-TRAY-CHANNEL-MONITOR-045`
- run_id: `20261006-tray-channel-monitor-045`
- checkpoint_id: `CP-TRAY-CHANNEL-MONITOR-045`
- owner: `Cursor Agent`
- created_at: `2026-10-05T23:49:00Z`
- updated_at: `2026-10-05T23:49:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `543064863d3316478763395929565f1fc78db27a`
- request: `부스터가 트레이로 들어가면 채널별 점수, 중앙상단 핑 표시 기능이 동작하지 않음`
- objective: 메인 창을 트레이로 숨겨도 활성화된 채널 측정, Windows 키 전체표, 게임 전경 상단 compact 표시를 유지한다.
- scope: 트레이 보조 창 정리 정책, 회귀 단위 테스트, 관련 자동 검사와 handoff.
- exclusions: 측정 공식·TCP EStats·foreground 판정 변경, 버전 증가, package·push·release 재배포.
- risk: `MEDIUM — 트레이 전환 중 native 보조 창 수명주기를 바꾸지만 채널 창 두 종류로 보존 범위를 제한하고 데이터·계약·보안 경계는 변경하지 않음`
- roles: `Coordinator=DEV=QA=Reviewer=Feature owner=Architecture owner=Documentation maintainer=Cursor Agent; SELF_REVIEW`
- dev_state: `IN_PROGRESS`
- qa_state: `PLANNED`
- review_verdict: `NOT_REVIEWED`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — 트레이 상태에서도 채널 점수 측정과 compact·expanded 표시 유지`
- feature_map: `no_change — 기존 기능 entry의 앱 실행 중 동작 범위가 의도이며 이번 변경은 그 범위를 복구`
- architecture_impact: `native tray auxiliary-window cleanup에서 채널 모니터링 창 수명주기를 main window visibility와 분리`
- architecture_contract: `no_change — 기존 독립 channel monitor thread·native window 경계를 수정 없이 준수`
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
