# 0.4.4 채널별 핑 패키지

- run_id: `20261005-release-044`
- checkpoint_id: `CP-RELEASE-044`
- owner: `Cursor Agent`
- created_at: `2026-10-04T21:29:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `d5b2ce28bacb8adc63c854a843eb172b9e53b971`
- roles: `Cursor Agent = Coordinator + DEV + QA + Feature owner + Architecture owner + Documentation maintainer`, `independent reviewer = 패키징 전 지정`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — 류트 서버 채널 endpoint 동기화·측정·게임 overlay와 입력 종료`, `FEAT-NOGIREM-TURBO-KEY — 정지 화면 제거 버튼 여백`
- feature_map: `updated — 0.4.4 source·로컬 signed package 검증 target과 미실행 수동 범위를 갱신`
- architecture_impact: `native auxiliary window, low-level input hook, 검증된 HWND 수명, signed installer·portable package`
- architecture_contract: `no_change — 기존 보조 창·입력 adapter·package 경계를 그대로 사용`

## 요청·권한·위험

- 요청: 채널별 핑 기능 설명 아래에 `이번 버전에서는 류트 서버의 채널만 확인 가능합니다.` 안내를 추가하고 overlay footer 아래 여백을 줄인다.
- 버전: 제품 버전을 `0.4.4`로 올리고 `VERSION_HISTORY.md`에 변경 기록을 추가한다.
- 패키징: Windows x64 설치형·포터블과 서명 update manifest를 생성·검증한다.
- 제외: GitHub Release 생성·자산 업로드, 배포, push, tag, production 활성화.
- risk: `HIGH — 저수준 전역 입력 hook과 native HWND 수명 변경을 포함한 signed 설치형·포터블 후보를 생성함`

## 계획·상태

- DEV: `DONE`
- QA: `COMPLETE_PASS — 자동 source·package 범위`
- review: `APPROVED — independent review`
- package: `COMPLETED`
- deployment: `IN_PROGRESS — user authorized GitHub v0.4.4 release at 2026-10-04T22:38:00Z`

1. `COMPLETED` — 안내 문구·footer 여백과 0.4.4 버전·변경 기록 반영
2. `COMPLETED` — 자동 검사와 독립 review
3. `COMPLETED` — signed installer·portable package와 manifest 검증
4. `COMPLETED` — handoff·최종 기록과 source commit

## 수용 기준

- `REL-044-001`: 고급 기능 채널 핑 설명 바로 아래에 류트 서버 한정 안내가 작은 보조 문구로 보인다.
- `REL-044-002`: overlay footer 아래 여백이 줄고 584×400 창·입력 종료·채널 배치는 유지된다.
- `REL-044-003`: package·Cargo 버전과 lockfile이 모두 0.4.4이며 변경 기록이 실제 기능 범위와 일치한다.
- `REL-044-004`: 자동 검사·독립 review·signed package·native contract·manifest self-verification이 통과한다.
- `REL-044-005`: 생성 산출물은 로컬에만 보관하고 배포·push하지 않는다.

## 증거 상태

- source review: `PASS — UI·version·lockfile·package 입력과 channel ping 전체 branch diff 독립 검토`
- behavioral verification: `PASS — 자동 source·local package contract 범위`, `NOT_RUN — 실제 게임 물리 입력·혼합 DPI·설치·제거 종단간`
- known gaps: 실제 마비노기 물리 입력·혼합 DPI 수동 QA와 설치·제거 종단간은 별도 확인이 필요함.

## package 전 QA·review

- `PASS` — Node `204/204`; 최초 병렬 부하 실행의 installer 3건 timeout은 단독 `9/9` 후 전체 단독 재실행 `204/204`로 재확인.
- `PASS` — backend `82 PASS`, `2 declared ignore`, parity `2/2`.
- `PASS` — native backend memory·affinity·network 통합. affinity `events.log`를 테스트 소유 임시 파일 정리 목록에 추가함.
- `PASS` — desktop `cargo check --locked`, 변경 파일 lint 0, `git diff --check`.
- reviewer: `generalPurpose agent 47916f5a-7c61-44f2-9476-30739bd8b0ff`
- review_mode: `INDEPENDENT_REVIEW`
- finding: `ISSUE-CP-020 RESOLVED — task·handoff 창 크기를 실제 584×400과 일치시킴`
- finding: `ISSUE-CP-021 RESOLVED — handoff의 package 미완료 기록을 로컬 signed package 완료·수동 QA와 release 미완료로 정정`
- finding: `ISSUE-CP-022 RESOLVED — build metadata와 두 signed manifest의 경로·SHA-256을 evidence에 추가`
- verdict: `APPROVED`, unresolved must-fix `0`
- review_target: `source commit 60f9b58cea09c4d1668af56c7cfda446211b4a51; 최종 재검토 tracked patch 97ab9e6d46c54c2e22d0be65694fb45e2dc3a8b727f4eedfb25778b635e83177`

## package·최종 disposition

- status: `COMPLETED`
- source_target: `60f9b58cea09c4d1668af56c7cfda446211b4a51`
- execution_target: `release/dioxus-0.4.4-2026-10-04T21-52-05-572Z`, Windows `10.0.26200` x64, Rust release·Dioxus Desktop/WebView2.
- installer: `nogirem-dioxus-setup-0.4.4.exe`, `9,540,158 bytes`, SHA-256 `1584e3fca5407df9582563a93ab0fc4f25c8face2a150b2f2bd9e01d6f5174dc`.
- portable: `nogirem-dioxus-portable-0.4.4.exe`, `7,770,752 bytes`, SHA-256 `6425e5d0de3833b114511b002069706a59f11e73d803bcde5e7ec20d97954697`.
- evidence: `evidence/package-0.4.4.md`
- `PASS` — release build, WebView2 signature 확인, NSIS `/WX`, 10 MB installer budget, packaged native service contract.
- `PASS` — installer·portable Ed25519 update manifest 생성·서명 self-verification; manifest payload size·SHA-256가 로컬 파일과 일치.
- `NOT_RUN` — 설치·제거·포터블 UI 실행, 실제 게임 전경·물리 입력·혼합 DPI, GitHub 원격 CSV.
- feature gate: `PASS for local package candidate — lifecycle PLANNED 유지, 미배포·수동 gap 명시`
- architecture gate: `PASS — 독립 review 승인, 새 경계·예외·위반 없음`
- review and QA gate: `PASS for automated/package scope`, independent review `APPROVED`, must-fix `0`.
- target and authorization gate: `PASS — source commit과 artifact hash 고정, 배포·push·tag·activation 없음`
- commit: `60f9b58cea09c4d1668af56c7cfda446211b4a51`; 최종 package 기록 commit은 별도 로컬 commit으로 남긴다.
