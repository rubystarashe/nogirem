# 0.4.2 QA 기록

- record_id: `QA-RELEASE-042-01`
- run_id: `20260929-release-042`
- checkpoint_id: `CP-RELEASE-042-01`
- owner: `cursor-agent-92251f36 as QA`
- updated_at: `2026-09-29T14:06:00Z`
- source_target: `86ca9261ab763e57477c7655d3f303b256d29bd8`, remote annotated tag `v0.4.2`
- execution_target: `Windows 10.0.26200 x64, Node 23.11.1, Rust locked dependencies, local debug/release build`
- QA_state: `COMPLETE_PASS for mandatory release scope`

## 시나리오

- `NET-FP-003` / 기본 IPv4 경로 부재: `PASS`
  - expected: Electron·Rust 모두 PowerShell stack 대신 `supported=false`, `disconnected=true`와 재연결 안내.
  - actual: source contract와 Node test가 두 경로를 확인했다.
  - dependency: Windows route cmdlet simulated by source contract; 실제 회선 단절은 `NOT_RUN`.
- `UPD-001` / backend 건강 확인: `PASS`
  - expected: main UI open 성공 뒤 worker·상태 조회·애니메이션과 무관하게 기록.
  - actual: service ordering contract와 startup smoke `passed=true`.
- `UPD-004` / 직접 제거 중 stale updater: `PASS`
  - expected: 보호 updater 작업 경로 process를 종료하고 caller 제거기는 보존.
  - actual: 임시 경로에 복사한 실제 `cmd.exe` process를 종료하고 caller 생존 확인.
- `UPD-005` / 종료 대기 중 늦게 생성된 process: `PASS`
  - expected: 최초 snapshot 이후 생성된 설치 폴더 process도 재탐색해 종료.
  - actual: 종료 script 실행 300ms 뒤 실제 process를 생성해 종료 확인.
- `UPD-006` / 0.4.1 updater 하위 호환: `PASS`
  - expected: 별도 인자 없이 새 installer를 실행한 검증된 부모 updater는 생존.
  - actual: 보호 cache 경로의 `updater.exe`가 installer 자식을 생성한 실제 parent 관계에서 updater와 caller 생존 확인.
- `UPD-UI-001` / 업데이트 UI: `PASS`
  - actual: debug Dioxus `--smoke-updater`, progress·downloaded·badge·noFocusSteal 확인.
- `PKG-001` / 실서명 패키징: `PASS`
  - actual: native service contract, release Rust build, Microsoft WebView bootstrap signature, NSIS `/WX`, Ed25519 manifest 생성·검증 통과.

## 자동 검사

- targeted Node: `PASS`, 39/39
- full Node: `PASS`, 198/198
- Rust backend: `PASS`, 56 executed with 1 declared `live_signed_update_download` ignore
- Rust desktop check: `PASS`
- IDE diagnostics: `PASS`, 변경 파일 오류 없음
- `git diff --check`: `PASS`
- independent review: `APPROVED`, `ISSUE-UPDATER-001 RESOLVED`
- full `npm run app:smoke`: `FAIL`
  - 기존에 기록된 base UI `Advanced structure failed: null`에서 첫 단계 종료.
  - 이번 변경과 관련된 startup 및 updater smoke를 각각 단독 실행해 모두 `PASS`.
  - 기존 실패를 passing으로 해석하지 않았으며 별도 알려진 UI smoke harness gap으로 유지한다.

## 릴리스 산출물

- bundle: `release/ready-v0.4.2-2026-09-29T13-57-45-990Z`
- installer: `303a9008f2bdd4a42c93cab7e9594e47f953617ab6d2d6a31ff4859f0d92ee04`
- portable: `fd956491fe940c9c02c8faa5782c03b0afbe395f24c19ea7f46dad0e15c57ad7`
- `update.json`: `6689bd276e4face6dc0a2f4793536a91dce9cecf3fc924b102c45c56226c7aac`
- `portable-update.json`: `b651cc1dc9cf6091da776701447930b4099b4b90ae1c629f398f1f52978cae61`
- Electron bridge와 turbo helper는 v0.4.1 공개 자산 digest와 일치한다.
- public release: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.2`
- remote verification: 두 공개 manifest의 Ed25519 signature를 embedded public key로 검증하고 설치형 9,459,721 bytes와 포터블 7,696,108 bytes를 실제 다운로드해 SHA-256 일치를 확인했다.

## 한계와 영향

- 외부 사용자 장치의 실제 0.4.1→0.4.2 앱 내 update는 `NOT_RUN`. 공개 서명·URL·실다운로드와 하위 호환 process 경로를 검증했다.
- 실제 인터넷 단절·복구 중 UI 수동 확인은 `NOT_RUN`.
- feature_impact: `FEAT-NOGIREM-NETWORK-FASTPING`, `FEAT-NOGIREM-UPDATE-LIFECYCLE`
- feature_map: `updated — docs/ai/wiki/feature-map.md에 시나리오와 evidence 반영`
- architecture_impact: `네트워크 writer와 updater/installer/helper 수명주기`
- architecture_contract: `updated — docs/ai/wiki/agent-friendly-architecture.md에 경계와 enforcement 반영`
