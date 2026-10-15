# 0.4.7 패키징

- run_id: `20261015-package-047`
- checkpoint_id: `CP-PACKAGE-047`
- owner: `Cursor Agent`
- created_at: `2026-10-15T13:13:01Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `bcd5205b1bcd421243b0e4c0b1bb75be523898f2`
- roles: `Cursor Agent = Coordinator + DEV + QA + Documentation maintainer`; package diff 독립 reviewer 별도 지정
- feature_impact: `FEAT-NOGIREM-TURBO-KEY — REMOVED 기능의 helper·약관 파일을 0.4.7 package·release asset에서 제외`
- feature_map: `updated — 배포 산출물 제외 근거를 추가할 예정`
- architecture_impact: `Windows package와 GitHub release asset assembly 입력에서 제거된 helper distribution 경계 폐쇄`
- architecture_contract: `updated — 제거 기능의 binary·resource 비배포 경계를 추가할 예정`

## 요청·범위·위험

- 요청: 현재 0.4.7 source를 패키징한다.
- 포함: 터보키 helper와 독립 약관 파일의 package/release asset 제외, 전체 자동 검사, signed 설치형·포터블·update manifest 생성과 검증, 로컬 source·기록 commit.
- 제외: push, tag, GitHub release, 배포, 설치·제거·포터블 실제 실행.
- package_notice_input: 사용자 `NOTICE.md`, SHA-256 `af1eb83b4ad8681314cf3123bf70f869f4d910434b401954a7c6825a8625f1f2`; package에는 포함하되 source commit에는 포함하지 않는다.
- unrelated_preserved: `NOTICE.md`, `data/`, `native/installer-cleanup/`.
- risk: `MEDIUM — 제품 runtime 동작은 바꾸지 않지만 제거 기능의 binary distribution 계약과 서명 배포 산출물을 변경한다. durable data·API·schema는 바꾸지 않는다.`
- authorization: package 생성과 로컬 commit만 허용. 외부 게시·배포는 금지.

## 계획·상태

1. `COMPLETED` — 제거된 터보키 helper·약관 파일을 package/release asset에서 제외
2. `COMPLETED` — 회귀 검사와 독립 review
3. `IN_PROGRESS` — package source commit과 exact target 고정
4. `NOT_STARTED` — signed 설치형·포터블·manifest 생성
5. `NOT_STARTED` — 산출물 hash·서명·payload·제거 경계 검증
6. `NOT_STARTED` — handoff·최종 보고 local commit

## 수용 기준

- package output과 release assembly에 `turbo-key-helper*.exe`, `TURBO_KEY_TERMS.md`가 없다.
- `native/turbo-key`, build script와 기존 구현 source는 보존한다.
- package app은 Rust service contract를 통과하고 Node/Electron backend payload를 포함하지 않는다.
- installer는 NSIS `/WX`, 10MB 제한과 WebView2 signature 검사를 통과한다.
- signed update manifest는 0.4.7 installer·portable size와 SHA-256에 일치하고 Ed25519 자체 검증을 통과한다.
- 실제 설치·실행은 `NOT_RUN`으로 기록한다.

## 현재 상태

- DEV: `READY_FOR_REVIEW`
- QA: `IN_PROGRESS`
- review: `APPROVED`, `INDEPENDENT_REVIEW`
- source_target: `base bcd5205b1bcd421243b0e4c0b1bb75be523898f2 + package distribution diff`
- execution_target: `NOT_CREATED`

## Source QA·review

- `PASS` — `npm test`: 206/206
- `PASS` — turbo·release asset targeted: 18/18
- `PASS` — package·release·prepare script syntax, `git diff --check`
- review_id: `REV-PACKAGE-047`
- review_mode: `INDEPENDENT_REVIEW`
- reviewer: `generalPurpose agent 135a3d60-5bf4-4467-98ff-8bca31dcdcb9`
- round_1: `CHANGES_REQUESTED — TURBO_KEY_TERMS.md 재포함을 막는 명시적 회귀 단언 누락`
- DEV_response: `package script에 TURBO_KEY_TERMS.md 문자열이 없음을 검사하는 단언 추가`
- round_2: `APPROVED — must-fix 해결, unresolved finding 0`
