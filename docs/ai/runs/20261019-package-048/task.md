# 0.4.8 패키징

- record_id: `TASK-PACKAGE-048-001`
- run_id: `20261019-package-048`
- checkpoint_id: `CP-PACKAGE-048`
- owner: `cursor-agent-92251f36`
- roles: `Coordinator=DEV=QA=Reviewer=Documentation maintainer=cursor-agent-92251f36`
- created_at: `2026-10-19T09:08:00Z`
- status: `IN_PROGRESS`
- DEV: `IN_PROGRESS`
- QA: `IN_PROGRESS`
- review_mode: `SELF_REVIEW`
- review_verdict: `NOT_REVIEWED`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — 균형 기본 CPU 모드가 포함된 0.4.8 배포 후보 생성`
- feature_map: `no_change — revision 33의 구현 설명은 정확하며 package evidence만 완료 후 갱신`
- architecture_impact: `none — 기존 signed installer·portable package 경로를 그대로 사용`
- architecture_contract: `no_change — revision 41의 package 경계가 현재 source와 일치`

## 요청·범위·위험

- request: 현재 source를 0.4.8 설치형·포터블로 패키징한다.
- included: 제품 버전 0.4.8 동기화, 전체 Node 검사, Rust compile, signed installer·portable·update manifest 생성, package 계약과 hash·서명 검증, 로컬 기록·commit.
- excluded: push, tag, GitHub Release, 배포, 실제 설치·제거·포터블 실행.
- package_notice_input: 사용자 `NOTICE.md`, SHA-256 `90b8803685fdb16e699c60b60d24d300738ff86bb8547a9a9eb481a2dc39cba3`, 1413 bytes. 패키지에는 포함하되 source commit에는 포함하지 않는다.
- unrelated_preserved: 사용자 수정 `NOTICE.md`, 비추적 `data/`.
- risk: `MEDIUM — signed 설치·업데이트 산출물을 생성하지만 외부 게시하지 않는다. runtime schema·durable writer·security boundary는 바뀌지 않는다.`
- authorization: package 생성과 로컬 commit만 허용. 외부 게시·배포 금지.

## 계획과 수용 기준

1. `IN_PROGRESS` — package/Cargo 버전과 lock을 0.4.8로 동기화하고 source target을 고정한다.
2. `PENDING` — `npm run package:win`으로 signed 설치형·포터블·manifest를 생성한다.
3. `PENDING` — version·size·SHA-256·Ed25519, Rust service contract, WebView2 signature, NSIS `/WX`, 10MB 제한과 필수/금지 payload를 검증한다.
4. `PENDING` — evidence, handoff, canonical freshness와 최종 보고를 갱신한다.

- acceptance: installer·portable 이름과 내부 `version.json`이 0.4.8이다.
- acceptance: signed manifest의 version·size·SHA-256와 실제 파일이 일치하고 Ed25519 자체 검증을 통과한다.
- acceptance: 사용자 NOTICE와 0.4.8 변경 기록이 포함된다.
- acceptance: Node/Electron backend, turbo helper·독립 약관, 제거된 game overlay payload가 포함되지 않는다.
- manual_QA: 실제 설치·실행·업데이트·게임은 `NOT_RUN`으로 기록한다.

## 현재 검증

- `PASS` — Node 213/213.
- `PASS` — Cargo version lock update와 desktop compile.
- source_base: `500e17444bc016d1283a82fe78e48ea77016b820`
- source_target: `0.4.8 version synchronization working target; local commit pending`
