# 0.4.5 공개 배포

- record_id: `RUN-20261005-RELEASE-045`
- run_id: `20261005-release-045`
- checkpoint_id: `CP-RELEASE-045`
- owner: `Cursor Agent`
- created_at: `2026-10-05T12:12:00Z`
- updated_at: `2026-10-05T12:12:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- intake_target: `base dff11ce080345a629e5e39fb21d0f8b3eadded01 + requester-edited VERSION_HISTORY.md`
- request: `0.4.5 패키징하고 배포`
- objective: 최신 0.4.5 source에서 서명 설치형·포터블과 update manifest를 생성·검증하고 source push, GitHub v0.4.5 Release 공개, 공개 재다운로드 검증을 완료한다.
- scope: 사용자 변경 기록 포함 source commit, 전체 자동 검사, Windows x64 signed package, Electron 0.3.19 migration bridge bundle, release asset 10개, push·tag·GitHub Release, 공개 hash·서명·routing 재검증.
- exclusions: 실제 사용자 PC 설치·제거·업데이트, 실제 게임·혼합 DPI 수동 QA.
- risk: `HIGH — 공개 source push와 서명된 실행 파일·업데이트 manifest를 GitHub Latest Release로 배포함`
- authorization: `사용자가 패키징과 배포를 명시적으로 승인함`
- roles: `Coordinator=DEV=QA=Documentation maintainer=Cursor Agent; release source review는 기존 REV-CHANNEL-QUALITY-045-R9 독립 승인 재사용, package/publication review는 SELF_REVIEW`
- dev_state: `IN_PROGRESS`
- qa_state: `PLANNED`
- review_verdict: `NOT_REVIEWED`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — 최근 가중 채널 점수, 조건부 재측정, 1분 TCP 경로 지표와 채널별 서버 지연 추측 UI를 0.4.5 공개 범위에 포함`
- feature_map: `no_change — canonical feature entry가 source target의 제품 동작과 수동 QA gap을 이미 반영`
- architecture_impact: `기존 signed package writer·Ed25519 update·Electron migration bridge·GitHub Release 경계 실행`
- architecture_contract: `no_change — 기존 fail-closed package·release assembly·update routing을 변경하지 않고 실행`
- stop_condition: `source push, v0.4.5 release asset 10개 공개, 후보/공개 SHA-256 일치, signed manifest·Electron bridge 재검증 또는 정확한 blocker 기록`

## Plan

1. 사용자 변경 기록을 보존한 committed source target을 만든다.
2. Node·Rust desktop·backend integration 자동 검사를 실행한다.
3. `npm run package:win`으로 최신 서명 설치형·포터블을 생성한다.
4. v0.4.4 공개 Electron migration bridge를 입력으로 release asset 10개를 조립·검증한다.
5. source를 origin/main에 push하고 v0.4.5 GitHub Release를 공개한다.
6. 공개 자산을 재다운로드해 후보 hash·Ed25519 manifest·Electron routing을 검증한다.
7. evidence·handoff·최종 상태를 기록하고 기록 commit을 push한다.
