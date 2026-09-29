# Agent-friendly architecture contract

- record_id: `ARCH-NOGIREM-001`
- owner: `nogirem maintainer`
- revision: `1`
- updated_at: `2026-09-29T13:49:00Z`
- updated_by: `cursor-agent-92251f36`
- source_reviewed_at: `2026-09-29T13:49:00Z`
- source_reviewed_by: `cursor-agent-92251f36`
- source_review_target: `main@caf15cd3bca07cd106a5688118bc2bd163b07540 + 20260929-release-042 working tree`
- source_review_evidence: `desktop/src`, `desktop/backend/src`, `desktop/*.nsi`, `scripts/package-dioxus.mjs`, release·installer tests
- behavior_verified_at: `2026-09-29T13:47:00Z`
- behavior_verified_by: `cursor-agent-92251f36`
- behavior_verification_target: `20260929-release-042 pre-release working tree`
- behavior_verification_environment: `Windows 10.0.26200 x64`
- behavior_verification_evidence: `npm test 198/198, backend Rust 56 executed with 1 declared live-network ignore, cargo check, independent review APPROVED`
- freshness_status: `CURRENT`
- freshness_reason: `0.4.2의 네트워크와 update 경계를 실제 소스·테스트에서 확인함`
- known_gaps: `아래 enforcement는 테스트와 Rust compiler 중심이며 별도 dependency graph lint는 없음`
- feature_impact: `FEAT-NOGIREM-NETWORK-FASTPING`, `FEAT-NOGIREM-UPDATE-LIFECYCLE`
- feature_map: `updated — feature-map.md의 두 기능과 연결됨`
- architecture_impact: `네트워크 writer, UI/backend 준비 신호, updater·installer·helper·uninstaller 수명주기`
- architecture_contract: `updated — 신규 정본으로 지정함`

## 계층과 확장 경로

1. `desktop/src`: Dioxus desktop host와 WebView 수명주기. 제품 설정을 직접 쓰지 않고 native service IPC를 사용한다.
2. `desktop/backend/src/service.rs`: renderer IPC 등록과 제품 service 조정. 무거운 기능 구현은 소유 module에 둔다.
3. `desktop/backend/src/<capability>.rs`: 기능별 authoritative logic과 Windows adapter 호출.
4. `desktop/backend/src/powershell`: PowerShell이 필요한 좁은 Windows adapter. 사용자 입력을 문자열로 직접 삽입하지 않는다.
5. `desktop/installer.nsi`, `desktop/portable.nsi`: 배포 bootstrap과 파일 설치·캐시 준비만 담당한다.
6. `scripts/package-dioxus.mjs`, `sign-update.mjs`: 재현 가능한 payload 조립과 매니페스트 서명 경로다.
7. `src`, `service`: 0.3.x Electron 전환 호환 경로이며 Rust 신기능의 정본이 아니다.

새 제품 기능은 capability module과 제한된 service 등록점을 사용한다. `service.rs`, NSIS root, update root에 기능별 분기를 늘릴 때는 등록·수명주기 조정만 두고 실제 동작은 소유 module이나 script에 둔다.

## 의존성 방향과 금지 경계

- renderer → preload → desktop RPC → backend service → capability module → Windows adapter 순서만 허용한다.
- backend module은 renderer DOM이나 WebView animation 상태를 제품 준비의 authoritative source로 사용하지 않는다.
- 설치·제거는 이미지 이름 전역 kill을 사용하지 않는다. 정규화된 설치 경로, 보호 updater 경로, caller·parent PID 검증 없이 프로세스를 종료해서는 안 된다.
- network writer는 조회한 adapter GUID 이외의 registry interface 경로를 쓰지 않는다.
- 포터블 배포 EXE를 실행 중 직접 덮어쓰지 않는다. 보호 staging, digest, backup과 launcher 종료 경로를 사용한다.
- 사용자 설정·녹화 파일은 설치 manifest와 제거 manifest에 포함하지 않는다.
- Electron 호환 코드는 새 Rust backend authoritative state의 두 번째 writer가 될 수 없다.

## durable data와 single writer

- network original snapshot: `network_manager.rs`만 생성·갱신하고 network restore 경로만 소비한다.
- update job state: `update_install.rs` helper가 `job.json/result.json`을 관리한다. service는 검증된 job의 `healthy.json`만 기록한다.
- `healthy.json` 준비 기준: backend RPC가 실행되고 main UI open 요청이 성공한 시점이다. worker, PowerShell 상태 조회, DXVK 조회, 시작 애니메이션 완료는 필수 조건이 아니다.
- portable cache marker: packager가 내부 payload digest를 만들고 launcher만 marker를 쓴다.
- update manifest: release signing script만 private key를 사용해 작성하며 앱은 embedded public key로 검증만 한다.
- installer uninstall registry·shortcut writer: NSIS 설치기 하나다.

## update·설치 transaction과 호환성

- manifest 서명과 payload SHA-256 검증 전에 설치를 시작하지 않는다.
- updater 작업은 migration mutex로 직렬화하고 보호 cache의 reparse·무결성 수준을 검증한다.
- 설치형은 파일·설정 backup 후 설치하고 건강 확인 실패 시 복원한다. 포터블은 동일 볼륨 원자 교체와 backup digest를 사용한다.
- 새 updater는 installer에 자신의 PID를 전달한다. 이전 0.4.1 updater는 인자가 없으므로 installer 종료 script가 검증된 parent `updater.exe` 하나를 자동 보존한다.
- 직접 설치·제거는 보호 updater 경로의 stale helper를 종료한다. 설치 폴더 프로세스가 정상 종료 중 다시 생길 수 있으므로 안정 구간까지 반복 조회한다.
- retry는 새 attempt identity와 현재 target을 사용하며 이전 attempt의 늦은 callback이 현재 상태를 덮지 못한다.

## API·schema와 생성물

- renderer IPC channel은 native contract test에서 등록 목록을 검증한다.
- `update.json`과 `portable-update.json`은 분리된 서명 envelope이며 filename, version, size, SHA-256을 포함한다.
- generated output: Rust web build, NSIS uninstall manifest, 설치형·포터블 EXE, signed manifests. authoritative input은 source, Cargo/npm lock, app assets, NSIS와 signing config다.
- 생성 명령: `npm run package:win`. 생성된 `release/`과 `target/` 산출물은 source commit에 포함하지 않는다.

## enforcement

- `npm test`: renderer/preload 계약, 설치·제거 실프로세스, 포터블·서명 정적 계약.
- `cargo test --locked --manifest-path desktop/backend/Cargo.toml`: backend 기능·update 보안 경계.
- `cargo check --locked --manifest-path desktop/Cargo.toml`: desktop/backend compile 경계.
- package script: native service contract, Node/Electron payload 부재, Microsoft bootstrap signature, NSIS warning-as-error, 파일 크기, signing key/public key 일치.
- CI gap: dependency direction과 process termination 경로를 전용 architecture linter로 검사하지 않는다. 관련 정적 테스트와 독립 리뷰가 현재 gate다.

## 예외

- active exceptions: `none`
- 새 예외는 stable ID, owner, 승인, 정확한 파일·edge·만료 또는 제거 조건, 보완 통제와 feature/architecture impact를 기록하기 전에는 적용할 수 없다.
