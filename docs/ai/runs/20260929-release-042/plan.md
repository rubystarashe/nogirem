# 0.4.2 구현·검증 계획

- record_id: `PLAN-RELEASE-042-01`
- run_id: `20260929-release-042`
- checkpoint_id: `CP-RELEASE-042-01`
- owner: `cursor-dev-92251f36`
- created_at: `2026-09-29T13:30:00Z`
- source_target: `nogirem/main@caf15cd3bca07cd106a5688118bc2bd163b07540`

## 체크포인트

1. `IN_PROGRESS` — 진단과 실제 소스의 영향 범위를 확정한다.
2. `NOT_STARTED` — Electron·Rust 네트워크 쿼리가 기본 경로 부재를 비예외 상태로 반환하게 한다.
3. `NOT_STARTED` — `Service::start`의 메인 UI 열기 성공 직후 건강 확인을 기록하고 애니메이션 완료 의존성을 제거한다.
4. `NOT_STARTED` — 설치·제거 PowerShell을 숨김 실행하고 updater 호출 설치를 표시하는 전용 인자를 추가한다.
5. `NOT_STARTED` — 직접 설치·제거에서 보호 캐시 helper를 종료하고 설치 폴더 프로세스를 안정 구간까지 반복 탐색한다.
6. `NOT_STARTED` — 정적 계약·실제 프로세스 재생성·helper 종료 회귀와 전체 Node/Rust 검증을 수행한다.
7. `NOT_STARTED` — 독립 코드 리뷰, 서명 패키징, 로컬 커밋, GitHub v0.4.2 배포와 원격 해시 검증을 수행한다.

## 구현 경계

- 정본 네트워크 writer: `src/network.mjs` 레거시 호환 경로와 `desktop/backend/src/network.rs`가 호출하는 PowerShell 스크립트. 대상 NIC는 기본 IPv4 경로와 adapter GUID로 식별한다.
- 정본 업데이트 writer: `desktop/backend/src/update_install.rs`. 설치형 updater만 `/NOGIREMUPDATE`를 전달한다.
- 설치·제거 종료 경로: `desktop/installer.nsi` → `scripts/stop-installed-app.ps1`. 설치 폴더와 보호된 `NogiremUpdater\updates` 경로 이외의 프로세스는 종료하지 않는다.
- 포터블 캐시 확인: `desktop/portable.nsi`. 프로세스 없음은 종료 코드 0, 발견은 전용 비정상 코드 2로 구분한다.
- 건강 확인: backend RPC와 메인 UI 열기 요청 성공을 준비 기준으로 사용하고 무거운 worker·상태 조회·시작 애니메이션은 기준에서 제외한다.

## 검증과 중단 조건

- `node --test test/network.test.mjs test/installer.test.mjs test/portable-release.test.mjs`
- 실제 임시 설치 폴더에서 늦게 생성된 프로세스와 보호 캐시 helper 종료 확인
- `npm test`
- `cargo test --locked --manifest-path desktop/backend/Cargo.toml`
- `cargo check --locked --manifest-path desktop/Cargo.toml`
- 패키징된 native service 계약, NSIS 패키징, Ed25519 매니페스트 서명
- 독립 리뷰에서 mandatory finding이 있거나 테스트·서명·공개 해시 검증이 실패하면 배포하지 않는다.
- 실제 사용자 컴퓨터의 인터넷 단절과 게임 내 체감은 로컬에서 재현할 수 없으므로 구조화 오류 및 적용 상태 계약까지만 자동 검증한다.

## 영향

- feature_impact: `FEAT-NOGIREM-NETWORK-FASTPING`, `FEAT-NOGIREM-UPDATE-LIFECYCLE`
- feature_map: `blocked — 구현과 검증 결과를 반영해 신규 정본을 생성할 예정`
- architecture_impact: `네트워크 adapter 선택, updater→installer 인자 계약, installer→PowerShell 프로세스 종료 경계, backend 건강 확인 경계`
- architecture_contract: `blocked — 검증된 경계와 금지된 전역 process kill을 신규 정본에 기록할 예정`
