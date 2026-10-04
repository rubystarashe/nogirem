# 0.4.4 채널별 핑 신규 기능 공지

- run_id: `20261005-notice-044`
- checkpoint_id: `CP-NOTICE-044`
- owner: `Cursor Agent`
- created_at: `2026-10-04T22:06:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `6d7a424f9de1aaf1c95f7e98d2ded9cc9fe17186`
- roles: `Cursor Agent = Coordinator + DEV + QA + Documentation maintainer`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — 0.4.4 신규 기능 소개 공지와 화면 이미지`
- feature_map: `no_change — 제품 동작·검증 범위는 바뀌지 않고 기존 기능을 소개함`
- architecture_impact: `none — 공지 Markdown·원격 정적 이미지·표시 크기만 변경`
- architecture_contract: `no_change — 기존 HTTPS 공지 이미지 경로를 사용`

## 요청·범위

- 요청: 제공된 채널별 핑 이미지를 GitHub raw 경로에서 불러오는 신규 기능 공지로 `NOTICE.md`를 작성하고 테스트 앱에서 확인한다.
- 포함: package에 포함되지 않는 `docs/notices/channel-ping-0.4.4.png`, 공지 문안, 이미지 전체 표시 스타일, 로컬 interactive preview.
- 게시 순서: 이미지 raw URL 확보를 위해 이미지와 run 상태만 먼저 push한다. `NOTICE.md`는 시각 확인 전 원격에 push하지 않는다.
- risk: `LOW — 제품 동작·데이터·통합 경계를 바꾸지 않는 공지 콘텐츠와 국소 표시 스타일이며 원격 공지 공개는 별도 확인 전 보류`

## 상태·수용 기준

- DEV: `READY_FOR_QA`
- QA: `COMPLETE_PASS — local interactive preview 범위`
- review: `APPROVED`, `review_mode: SELF_REVIEW`
- publishing: `IMAGE_ONLY_COMPLETED`, `NOTICE_NOT_PUBLISHED`, `LOCAL_PACKAGE_AUTHORIZED`

- 이미지 URL은 HTTPS raw GitHub 주소이며 package script 입력에 포함되지 않는다.
- 공지는 류트 서버 한정, 고급 기능 활성화, Windows 키 표시, 입력 시 닫기와 측정값 의미를 정확히 설명한다.
- 제공된 이미지가 잘리지 않고 공지 폭에 맞게 표시된다.
- 테스트 앱이 production 공지 renderer로 `NOTICE.md`를 열고 screenshot·구조 결과를 남긴다.

## 구현·검증

- image_remote: `https://raw.githubusercontent.com/rubystarashe/nogirem/main/docs/notices/channel-ping-0.4.4.png`
- image_remote_commit: `05e6d59da8970a39a0cac07993f0278655b4a4c6`
- image_remote_check: `HTTP 200`, `image/png`, `561,087 bytes`
- notice: 0.4.4 채널별 핑 목적, 고급 기능 활성화, Windows 키 표시, 모든 후속 입력 종료, 류트 서버 한정을 안내함.
- style: 공지 폭 100%, 원본 `958:668` 비율, `object-fit: cover`로 빈 여백 없이 표시함.
- fixture: 신규 `application:get-channel-ping-setting` route 누락으로 첫 preview에 표시된 오류를 수정함. 실제 production IPC 오류가 아니라 smoke fixture 결손이었음.
- `PASS` — Node full suite `204/204`
- `PASS` — Rust notice Markdown remote image `1/1`
- `PASS` — desktop `cargo check --locked`, lint 0, `git diff --check`
- `PASS` — production renderer interactive preview, overflow `0`, title·본문 일치, 앱을 열린 상태로 유지함.
- evidence: `evidence/notice-preview.json`, SHA-256 `ca62dea44f3acf4d0d0e77066c71611035dd8770054fe8deecaacfbb72d231fa`
- evidence: `evidence/notice-preview.png`, SHA-256 `4ffe2ddc182c0bbb30c622a5f2a872434182acdb01bc48906d1fdea1bcbc849c`
- limitation: 공지 본문은 아직 origin/main에 push하지 않아 사용자에게 노출되지 않음. 현재 테스트 앱 화면 확인 뒤 게시 여부를 결정함.
