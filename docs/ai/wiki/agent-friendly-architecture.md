# Agent-friendly architecture contract

- record_id: `ARCH-NOGIREM-001`
- owner: `nogirem maintainer`
- revision: `5`
- updated_at: `2026-10-02T15:30:00Z`
- updated_by: `cursor-agent-92251f36`
- source_reviewed_at: `2026-10-02T14:50:00Z`
- source_reviewed_by: `cursor-agent-92251f36`
- source_review_target: `SOURCE-MANIFEST-DXVK-043-R4, base 8f365e1f0b4566f0c41da87856e9a290b59a0d28, manifest SHA-256 8ad94541aa2daef8b158ac3b309cd93a8e6238422d99b02c708f9762ff75efba`
- source_review_evidence: `desktop/backend/src/dxvk.rs`, `dxvk_manager.rs`, DXVK Rust·Node 회귀 테스트
- behavior_verified_at: `2026-10-02T15:30:00Z`
- behavior_verified_by: `cursor-agent-92251f36`
- behavior_verification_target: `source commit 567ab935aad7d69476829b1e6a2e5a64d6c0eec7 + GitHub v0.4.3 public assets replaced at 2026-10-02T15:25Z`
- behavior_verification_environment: `Windows 10.0.26200 x64`
- behavior_verification_evidence: `backend 71 PASS·2 declared ignore와 integration 2/2, DXVK 14/14, Node 203/203, desktop locked check, signed package, release assembly와 공개 재다운로드 SHA-256·Ed25519 검증`
- freshness_status: `CURRENT`
- freshness_reason: `0.4.3 DXVK 저장·적용 transaction과 game path 경계를 source·자동·공개 배포 검증에 맞춰 반영함`
- known_gaps: `별도 dependency graph lint가 없고 실제 게임 폴더·보안 제품 격리 수동 QA는 NOT_RUN`
- feature_impact: `FEAT-NOGIREM-DXVK — 저장소와 게임 폴더 DLL의 검증·원자 교체·오류 경계`
- feature_map: `updated — feature-map.md의 DXVK 기능과 연결됨`
- architecture_impact: `DXVK manager path validation, file writer ownership, handle-based replace transaction`
- architecture_contract: `updated — DXVK download·file transaction 경계를 추가함`

## 계층과 확장 경로

1. `desktop/src`: Dioxus desktop host와 WebView 수명주기. 제품 설정을 직접 쓰지 않고 native service IPC를 사용한다.
2. `desktop/backend/src/service.rs`: renderer IPC 등록과 제품 service 조정. 무거운 기능 구현은 소유 module에 둔다.
3. `desktop/backend/src/<capability>.rs`: 기능별 authoritative logic과 Windows adapter 호출.
4. `desktop/backend/src/powershell`: PowerShell이 필요한 좁은 Windows adapter. 사용자 입력을 문자열로 직접 삽입하지 않는다.
5. `desktop/installer.nsi`, `desktop/portable.nsi`: 배포 bootstrap과 파일 설치·캐시 준비만 담당한다.
6. `scripts/package-dioxus.mjs`, `sign-update.mjs`: 재현 가능한 payload 조립과 매니페스트 서명 경로다.
7. `scripts/release-assets.mjs`, `prepare-release.mjs`: 서명된 원본을 자동 업데이트 호환 이름과 사용자 권장 alias로 조립하고 Electron feed·sha512·blockmap을 검증한다.
8. `src`, `service`: 0.3.x Electron 전환 호환 경로이며 Rust 신기능의 정본이 아니다.

새 제품 기능은 capability module과 제한된 service 등록점을 사용한다. `service.rs`, NSIS root, update root에 기능별 분기를 늘릴 때는 등록·수명주기 조정만 두고 실제 동작은 소유 module이나 script에 둔다.

## 의존성 방향과 금지 경계

- renderer → preload → desktop RPC → backend service → capability module → Windows adapter 순서만 허용한다.
- backend module은 renderer DOM이나 WebView animation 상태를 제품 준비의 authoritative source로 사용하지 않는다.
- 설치·제거는 이미지 이름 전역 kill을 사용하지 않는다. 정규화된 설치 경로, 보호 updater 경로, caller·parent PID 검증 없이 프로세스를 종료해서는 안 된다.
- network writer는 조회한 adapter GUID 이외의 registry interface 경로를 쓰지 않는다.
- 포터블 배포 EXE를 실행 중 직접 덮어쓰지 않는다. 보호 staging, digest, backup과 launcher 종료 경로를 사용한다.
- 사용자 설정·녹화 파일은 설치 manifest와 제거 manifest에 포함하지 않는다.
- Electron 호환 코드는 새 Rust backend authoritative state의 두 번째 writer가 될 수 없다.
- UI runtime polling은 한 helper 요청 실패로 영구 종료하지 않는다. 실패한 기능의 `running`·`gameActive`를 false로 보정하고 다음 주기에 재조회한다.
- affinity 실행 상태는 상태 파일 timestamp만으로 판정하지 않고 해당 `helperPid`의 생존과 `helperStartedAt`이 현재 프로세스 시작 시각과 일치하는지 확인한다.
- 터보키는 임의의 `Client.exe` 이름만으로 입력하지 않는다. 정본 설정과 동일한 명시적 게임 폴더 또는 sibling launcher를 검증해야 한다.
- DXVK는 확인되지 않은 게임 경로나 임의 URL에 파일을 쓰지 않는다. manager가 실제 `Client.exe`와 게임 미실행 상태를 확인하고 capability module이 digest·x64 PE·최종 파일 hash를 검증해야 한다.

## durable data와 single writer

- network original snapshot: `network_manager.rs`만 생성·갱신하고 network restore 경로만 소비한다.
- update job state: `update_install.rs` helper가 `job.json/result.json`을 관리한다. service는 검증된 job의 `healthy.json`만 기록한다.
- `healthy.json` 준비 기준: backend RPC가 실행되고 main UI open 요청이 성공한 시점이다. worker, PowerShell 상태 조회, DXVK 조회, 시작 애니메이션 완료는 필수 조건이 아니다.
- portable cache marker: packager가 내부 payload digest를 만들고 launcher만 marker를 쓴다.
- update manifest: release signing script만 private key를 사용해 작성하며 앱은 embedded public key로 검증만 한다.
- installer uninstall registry·shortcut writer: NSIS 설치기 하나다.
- game path state: affinity helper만 `%APPDATA%\마비노기 렘 부스터\game\path.json`을 쓰며 `Environment`와 입력 기능이 읽는다. 터보키의 허용 판단에는 사용하지 않는다.
- turbo helper installation: `inputs.rs`가 helper version·manifest·binary 교체를 조정하고 native helper는 전달받은 설정과 game path state를 소비한다.
- DXVK state와 DLL: `dxvk.rs` install/apply 경로만 `%APPDATA%\마비노기 렘 부스터\vulkan`의 versioned DLL·`current.json`과 검증된 게임 폴더의 `d3d9_dxvk.dll`을 쓴다.

## 실시간 helper 상태와 게임 경로

- affinity worker는 `status.json`에 `helperPid`, `updatedAt`, `running`, `gameActive`를 함께 기록한다. backend는 최신 timestamp와 실제 PID 생존을 모두 만족할 때만 실행 중으로 노출한다.
- `helperStartedAt`이 없는 신선한 0.4.2 status와 살아 있는 PID는 migration 대상으로만 인정한다. 새 helper가 lock을 경쟁하기 전에 기존 control 경로로 중단하고 동일 PID·시작 시각 instance의 종료를 확인한다.
- UI는 2초 runtime polling을 유지한다. 개별 affinity/memory 오류는 사용자에게 한 번 표시할 수 있지만 polling task 자체를 종료하거나 이전 `gameActive=true`를 유지하지 않는다.
- 게임이 실행 중이 아니어도 helper는 대기 상태일 수 있으며 사용자의 부스트 중단·복원 조작을 차단하지 않는다.
- 터보키는 전경 프로세스가 `Client.exe`이고 상위 폴더가 `Mabinogi`, `Mabinogi_Test`, `마비노기`, `Nexon` 중 하나이거나 같은 폴더에 `Mabinogi.exe`가 있을 때만 반복 입력한다.
- 사용자 쓰기 가능한 runtime game path 파일은 터보키 허용 목록으로 신뢰하지 않는다. 새 게임 폴더 지원은 source 정본과 회귀 테스트를 함께 변경한다.

## DXVK download·file transaction

- `dxvk_manager.rs`는 다운로드 전에 실제 `Client.exe`, 허용 폴더명 또는 sibling launcher, 파일·모든 상위 경로의 reparse 여부와 canonical parent를 확인한다. 다운로드가 끝난 뒤 게임 실행 여부와 canonical 대상 경로를 교체 직전에 다시 확인하며 달라졌으면 적용하지 않는다.
- `dxvk.rs`는 고정 GitHub release 주소, 공개 SHA-256, 제한 크기, x64 PE를 확인한 뒤에만 versioned DLL을 저장한다.
- 저장소와 게임 폴더의 디렉터리·임시 파일·기존 파일 handle identity를 유지·재확인하고, 같은 디렉터리의 UUID 임시 파일에 기록·flush·hash 검증한다. 기존 파일은 별도 UUID backup에 flush·hash 검증하고 staged handle의 `SetFileInformationByHandle(FileRenameInfo)`로 기존 target을 교체한 뒤 최종 hash를 다시 확인한다.
- 교체 전 실패하면 기존 파일을 유지하며, 교체 뒤 내부·외부 검증 실패는 검증된 backup을 복원·재검증한다. 원본이 없으면 실패한 새 target을 제거한다. 임시 파일과 성공적으로 끝난 backup은 정리하되 복구 실패 시 검증된 backup은 수동 복구를 위해 보존한다.
- `NotFound`는 Windows 보안·백신 격리 가능성, access denied는 보안 차단·권한·파일 사용, sharing violation은 실행 중인 프로그램 사용으로 구분해 사용자에게 작업 단계와 함께 알린다.
- 오류 메시지와 진단에는 사용자 전체 게임 경로를 포함하지 않는다. 보안 정책·백신 예외는 앱이 자동으로 변경하지 않는다.

## update·설치 transaction과 호환성

- manifest 서명과 payload SHA-256 검증 전에 설치를 시작하지 않는다.
- updater 작업은 migration mutex로 직렬화하고 보호 cache의 reparse와 Win32 mandatory-label ACL을 검증한다. PowerShell SDDL 문자열 표현은 신뢰 판정에 사용하지 않으며 label authority, NO_WRITE_UP 정책과 High/System RID를 모두 요구한다.
- 설치형은 파일·설정 backup 후 설치하고 건강 확인 실패 시 복원한다. 포터블은 동일 볼륨 원자 교체와 backup digest를 사용한다.
- 새 updater는 installer에 자신의 PID를 전달한다. 이전 0.4.1 updater는 인자가 없으므로 installer 종료 script가 검증된 parent `updater.exe` 하나를 자동 보존한다.
- 직접 설치·제거는 보호 updater 경로의 stale helper를 종료한다. 설치 폴더 프로세스가 정상 종료 중 다시 생길 수 있으므로 안정 구간까지 반복 조회한다.
- retry는 새 attempt identity와 현재 target을 사용하며 이전 attempt의 늦은 callback이 현재 상태를 덮지 못한다.

## API·schema와 생성물

- renderer IPC channel은 native contract test에서 등록 목록을 검증한다.
- `update.json`과 `portable-update.json`은 분리된 서명 envelope이며 filename, version, size, SHA-256을 포함한다.
- 배포된 Rust verifier가 요구하는 `nogirem-dioxus-*`는 manifest용 canonical asset이며 짧은 이름은 byte-identical 수동 다운로드 alias다. Electron legacy 이름은 `latest.yml`의 path·두 sha512·필수 blockmap과 함께 변경한다.
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
