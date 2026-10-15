# 0.4.6 공개 배포

- record_id: `RUN-20261015-RELEASE-046`
- run_id: `20261015-release-046`
- checkpoint_id: `CP-RELEASE-046`
- owner: `Cursor Agent`
- created_at: `2026-10-15T06:35:00Z`
- updated_at: `2026-10-15T06:35:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `9c47fa00e228691e3c49505ff2a602f7de7c4aa6`
- request: `커밋하고 배포`
- objective: 사용자 최종 공지를 포함한 exact 0.4.6 source에서 설치형·포터블과 update manifest를 재생성·검증하고 source push, v0.4.6 tag와 GitHub Release 공개, 공개 자산 재검증을 완료한다.
- scope: 사용자 `NOTICE.md` commit, targeted notice tests, signed Windows x64 package 재생성, Electron 0.3.19 migration bridge 포함 release asset 10개 조립, source push·tag·GitHub Release, 공개 hash·서명·routing 검증.
- exclusions: 실제 사용자 PC 설치·제거·자동 업데이트·포터블 실행, 실제 게임·혼합 DPI 수동 QA.
- risk: `HIGH — source push와 서명된 실행 파일·update manifest를 공개 GitHub Release로 배포`
- authorization: `사용자가 commit과 배포를 명시적으로 승인함`
- roles: `Coordinator=DEV=QA=Reviewer=Documentation maintainer=Cursor Agent; package·publication은 SELF_REVIEW; 독립 reviewer 없음`
- dev_state: `IN_PROGRESS`
- qa_state: `PLANNED`
- review_verdict: `NOT_REVIEWED`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — tray 상태 fix 공개; FEAT-NOGIREM-GAME-OVERLAY — REMOVED source와 공지 공개`
- feature_map: `no_change — canonical entries가 source와 lifecycle을 정확히 반영`
- architecture_impact: `기존 signed package writer·Ed25519 update·Electron migration bridge·GitHub Release 경계 실행`
- architecture_contract: `no_change — package·update·image recognition 금지 경계 변경 없음`
- stop_condition: origin/main과 v0.4.6 tag가 exact source를 가리키고 공개 자산 10개가 후보 hash·signed manifest·Electron routing 재검증을 통과하거나 정확한 blocker를 기록한다.

## Plan

1. 사용자 최종 공지와 release run을 commit해 exact source target을 만든다.
2. 공지 렌더링 회귀 검사 후 0.4.6 서명 패키지를 exact source에서 재생성한다.
3. v0.4.5 공개 Electron migration bridge를 사용해 release asset 10개를 조립·검증한다.
4. source를 origin/main에 push하고 annotated v0.4.6 tag와 공개 GitHub Release를 만든다.
5. 공개 자산 10/10을 재다운로드해 후보 SHA-256, Ed25519 manifest와 Electron bridge routing을 재검증한다.
6. evidence·handoff·final report를 commit하고 origin/main에 push한다.
