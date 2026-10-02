# 0.4.3 부스트 상태·테스트 서버 터보키 수정 배포

- run_id: `20261002-release-043`
- checkpoint_id: `CP-RELEASE-043`
- owner: `Cursor Agent`
- created_at: `2026-10-02T12:22:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `1292f62c084200ec41201e1792d212ad58cc0569`
- roles: `Cursor Agent = Coordinator + DEV + QA + Documentation maintainer`, `independent reviewer = 배포 전 지정`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — helper 종료·게임 미실행 상태 표시와 중단 조작 복구`, `FEAT-NOGIREM-TURBO-KEY — 테스트 서버 Client.exe 전경 입력 지원`
- feature_map: `updated — 두 기능 identity와 0.4.3 동작·검증 범위를 정본에 추가`
- architecture_impact: `desktop runtime polling, affinity helper PID·시작 시각 identity, 터보키의 명시적 게임 폴더 허용 경계`
- architecture_contract: `updated — helper 상태 identity와 테스트 서버 경로·오류 복구 경계를 추가`

## 요청과 범위

- 원 요청: 게임이 꺼졌는데도 실시간 부스트중으로 남고 조작되지 않는 문제를 개선한다.
- 추가 요청: 최적화는 적용되지만 테스트 서버에서 터보키가 동작하지 않는 경로 판별 문제를 개선한다.
- 배포 요청: 두 수정사항을 0.4.3으로 패키징하고 문제가 없으면 GitHub Release로 배포한다.
- 포함: Rust/Dioxus UI·backend, 터보키 helper 0.1.8, 버전·변경 기록·정본 문서, 서명 설치형·포터블·manifest와 GitHub v0.4.3 자산.
- 제외: Electron 제품 동작 변경, tag/history rewrite, main push, 실제 사용자 PC 테스트 서버 수동 입력 검증.

## 위험 분류와 계획

- risk: `HIGH — 실행 중 CPU affinity 상태 복구, 저수준 키보드 hook 대상 판별, 서명 자동 업데이트 배포를 함께 변경`
- DEV: `READY_FOR_REVIEW`
- QA: `COMPLETE_PASS for mandatory automated and package scope`
- review: `APPROVED`
- release: `READY_TO_PUBLISH`

1. `COMPLETED` — helper 생존·poll 오류·게임 미실행 중 중단 조작 수정
2. `COMPLETED` — 정본 설정과 동일한 테스트 서버 폴더를 터보키 대상 판별에 추가
3. `COMPLETED` — 0.4.3/터보키 0.1.8 버전, 기능 지도와 architecture contract 갱신
4. `COMPLETED` — 전체 자동 검사, helper build, 독립 review
5. `IN_PROGRESS` — signed package 생성·검증 완료, source commit 진행
6. `NOT_STARTED` — GitHub v0.4.3 release·공개 다운로드 검증, handoff·최종 상태

## 수용 기준과 시나리오

- `BOOST-001`: affinity helper PID가 종료되면 30초 stale status를 실행 중으로 표시하지 않는다.
- `BOOST-002`: runtime 조회 한 번 실패해도 2초 polling이 영구 종료되지 않고 이후 상태를 다시 반영한다.
- `BOOST-003`: 게임 미실행 대기 상태에서도 사용자가 부스트를 중단할 수 있다.
- `BOOST-004`: 살아 있는 0.4.2 affinity helper는 control 경로로 정상 종료한 뒤 0.4.3 helper로 교체한다.
- `TURBO-001`: `Mabinogi_Test\Client.exe`를 전경 게임 대상으로 인정한다.
- `TURBO-002`: 정본에 없는 폴더의 다른 `Client.exe`는 전경이어도 거부한다.
- `REL-043-001`: Rust·Node 테스트, desktop compile, package contract, Ed25519 manifest와 공개 payload hash가 통과한다.

## 중단 조건

- 새 affinity 오탐·다른 Client.exe 입력 허용, helper version/asset 불일치, 필수 테스트 실패, 독립 review mandatory finding, 서명키 불일치, package 또는 공개 digest 불일치 시 배포를 중단한다.
- 실제 테스트 서버 수동 입력은 환경 부재로 `NOT_RUN`이며 자동 경로 판별 검증 범위보다 넓은 행동 검증을 주장하지 않는다.

## 구현·QA·리뷰 상태

- 구현: affinity status가 `helperPid`와 `helperStartedAt`을 기록하고 backend가 동일 PID·시작 시각 생존을 확인한다. polling 오류는 해당 runtime을 비실행으로 보정하되 loop를 유지하며, 게임 대기 중에도 부스트 중단을 허용한다.
- 호환성: `helperStartedAt`이 없는 신선한 0.4.2 helper는 control로 중단하고 동일 process instance 종료를 확인한 뒤 0.4.3 helper를 시작한다.
- 터보키: helper 0.1.8이 `Mabinogi_Test\Client.exe`를 명시적으로 허용하며 허용 폴더 밖의 `Client.exe`는 계속 거부한다.
- `PASS` — Node `203/203`
- `PASS` — backend `60 PASS`, `2 declared ignore`, integration `2/2`
- `PASS` — turbo-key `14/14`
- `PASS` — desktop `cargo check --locked`
- `PASS` — 변경 파일 IDE lint, `git diff --check`
- `PASS` — signed package, native service contract, WebView2 서명, NSIS `/WX`, Ed25519 manifest 생성·검증
- `PASS` — release asset assembly `3/3`, canonical/alias byte identity와 Electron 0.3.19 bridge 보존
- `FAIL / 기존 harness 제한` — full desktop smoke가 `report.json` 생성 전 종료되어 결과를 수집하지 못했다. 대기 버튼 계약은 source 회귀와 desktop compile로 검증했으며 실제 UI 수동 검증은 `NOT_RUN`.
- `NOT_RUN` — 실제 테스트 서버 반복 입력, 실제 0.4.2 helper 프로세스를 둔 migration 종단간.

## 독립 review

- reviewer: `cursor subagent 2a29cbc2-5012-411f-b313-ada3ad306ba6`
- review_mode: `INDEPENDENT_REVIEW`
- findings: `ISSUE-043-001~005 RESOLVED`, unresolved mandatory finding `0`
- verdict: `APPROVED`
- 주요 수정: PID 재사용 방지, 사용자 쓰기 runtime path 신뢰 제거, smoke 계약 갱신, 검증 수치 정합성, 살아 있는 0.4.2 helper migration.
- limitations: 실제 legacy helper migration·테스트 서버 입력·desktop smoke·공개 payload는 reviewer가 실행하지 않음.

## 배포 후보

- build: `release/dioxus-0.4.3-2026-10-02T12-56-11-066Z`
- candidate: `release/ready-v0.4.3-2026-10-02T13-00-00-236Z`
- installer canonical/alias SHA-256: `0eed1ea9bbdd12c9274cbc40c4166207fdb6c9473315b052d2d8cec295bced9c`
- portable canonical/alias SHA-256: `91f8415f09c508528c763babc7bfda5bd0fc7af3f4d9b682b52ae2f5d2c35756`
- turbo-key 0.1.8 SHA-256: `5b1e79528b615cd02ca7a7ec59a45a0f0942e58130c8ac0bc6540fb4ccd78936`
- update manifest SHA-256: `b6514d0c1e9c2d6c9a587cb9177005d05a71782aa17665ddbf8f8fe0b84effa9`
- portable manifest SHA-256: `f13b402d2e9af306f20f421cd15326d298c7fb4658ee3928d1edda52275d1d79`
