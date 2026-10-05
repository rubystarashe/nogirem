# Agent-friendly architecture contract

- record_id: `ARCH-NOGIREM-001`
- owner: `nogirem maintainer`
- revision: `14`
- updated_at: `2026-10-05T11:47:00Z`
- updated_by: `cursor-agent-92251f36`
- source_reviewed_at: `2026-10-05T11:47:00Z`
- source_reviewed_by: `cursor-agent-92251f36`
- source_review_target: `source commit cf607c8f2506a1c7a903e8ffbb5e36773a016546, base 3172d6d216058beef01521ce0f85077f0b15ab3d`
- source_review_evidence: `recency weights, conditional endpoint confirmation and zero-value renderer boundaries; REV-CHANNEL-QUALITY-045-R9 INDEPENDENT_REVIEW APPROVED`
- behavior_verified_at: `2026-10-05T11:47:00Z`
- behavior_verified_by: `cursor-agent-92251f36`
- behavior_verification_target: `source commit cf607c8f2506a1c7a903e8ffbb5e36773a016546`
- behavior_verification_environment: `Windows 10.0.26200 x64 automated source target`
- behavior_verification_evidence: `channel ping backend 33/33, overlay interface 3/3, desktop check, targeted rustfmt, diff check, IDE diagnostics; Chromium all-zero·mixed-positive renderer 확인`
- freshness_status: `CURRENT`
- freshness_reason: `실시간 게임 서버 모니터링의 분산 측정·실제 TCP read·compact/expanded 경계를 source와 일치시킴`
- known_gaps: `별도 dependency graph lint가 없고 실제 endpoint 조건부 추가 연결·native WebView 0값 숨김·actual HWND 비전경 숨김·DPI·최신 package 수동 QA는 NOT_RUN`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — 안정성 판정·실제 연결·상시 compact 오버레이`
- feature_map: `updated — feature-map.md의 신규 채널 핑 identity와 연결됨`
- architecture_impact: `channel CSV/cache single writer, staggered TCP worker, owner-PID TCP EStats collector, low-level input hooks, compact/expanded click-through window`
- architecture_contract: `updated — 채널 핑 network·input·window 경계를 추가함`

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
- 채널 핑은 사용자 URL·hostname·endpoint를 받지 않는다. 고정 HTTPS source의 제한 크기 CSV를 전부 검증한 뒤에만 cache와 runtime endpoint를 교체한다.
- 실시간 점수 창은 게임 프로세스에 주입하거나 Windows 키를 차단하지 않는다. 실제 실행 경로가 정본 게임 조건을 충족한 전경 PID가 소유하고 CSV endpoint와 일치하는 established TCP 연결만 조회하며 비활성·click-through 상태를 유지한다. 상단 창은 게임 전경 또는 전체표가 열린 Windows shell 전경에서만 표시하고 일반 앱 전경에서는 마지막 게임 bounds를 재사용하지 않는다.

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
- channel ping setting·cache: `channel_ping.rs`만 `%APPDATA%\마비노기 렘 부스터\channel-ping\setting.json`과 검증된 `channel.csv`를 쓴다. 동일 module이 연결 시험 표본과 실제 5-tuple별 `SmoothedRtt + RttVar` TCP 추정값을 계산하고, 최근 유효 counter보다 재전송·TCP 시간초과·수신 중복 ACK·혼잡 신호 중 하나라도 증가한 표본에는 `SmoothedRtt + 4×RttVar`를 적용한다. 최근 1분 추정 표본 평균·최대, 네 counter delta 표본과 최근 유효 counter 기준은 메모리에만 두며 overlay·UI는 status IPC로 읽는다. 추정 최대는 원시 패킷 RTT 최대나 게임 처리·입력·렌더링 지연을 뜻하지 않는다. rolling outcomes·실제 연결·window mode는 지속하지 않는다.

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

## 실시간 핑 network·input·auxiliary window

- 시작할 때 고정 GitHub raw HTTPS URL을 한 번 확인한다. 응답은 64KiB 이하 UTF-8이고 `채널,IP:포트` header, 1부터 연속된 unique channel, 공개 unicast IPv4 literal, 1~65535 port를 모두 만족해야 한다. 하나라도 어긋나면 cache를 쓰지 않고 마지막 유효 cache 또는 package fallback을 유지한다.
- endpoint가 없는 명시 channel은 `정보 없음`이며 측정 실패가 아니다. endpoint가 있는 채널은 60초 주기 안에서 하나씩 분산 TCP 연결하고 최근 20회 전체 결과 중 성공 표본의 중앙값·최근 가중 평균 절대편차·최대 변동과 같은 창의 실패율을 메모리에 보존한다. 기존 성공 3개 이상에서 새 결과가 중앙값보다 0.5ms 이상 높고 기존 최대 편차의 3배를 넘을 때만 50ms 간격으로 2회 추가 연결하며, 세 결과 중앙값 하나를 기록한다. 평상시에는 단일 연결만 수행하고 반복 지연은 확인 재측정에서 보존한다. 연결 실패가 하나라도 있으면 시작 확인 시각부터 최대 1시간에 한 번만 CSV를 재검사한다.
- 정본 게임이 전경일 때만 1초 주기로 owner-PID TCP table을 읽는다. 검증 중 연 process handle을 조회 완료까지 유지해 PID 재사용을 차단하고, established remote IP:port가 CSV endpoint와 정확히 일치하는 5-tuple만 채널 후보로 인정한다. 동일한 이전 5-tuple을 우선하며 그 밖의 복수 후보가 모호하면 표시하지 않는다.
- 선택한 connection에 한해 Windows TCP EStats 수집을 활성화하고 smoothed RTT·RTT 변동과 최근 유효 poll 사이 `PktsRetrans`·`Timeouts`·`DupAcksIn`·`CongSignals` 증가를 TCP 추정값에 반영한다. 공개 status는 누적값, poll delta와 최근 1분 합계인 `*Recent`를 분리하며 UI는 `packetsRetransmittedRecent`, `tcpTimeoutsRecent`, `duplicateAcksReceivedRecent`, `congestionSignalsRecent`만 읽는다. 일시적으로 RTT가 없는 poll은 네 counter 기준을 덮지 않고, counter 감소는 reset으로 처리해 해당 delta를 0으로 둔 뒤 새 기준을 저장한다. EStats 활성화·조회 실패는 상태에 명시하며 패킷 캡처·게임 메모리 접근·주입은 금지한다.
- 기능이 꺼져 있으면 TCP 측정·실제 연결 조회를 하지 않고 저수준 input hook도 설치하지 않는다. 기능이 켜진 동안 전용 message thread의 `WH_KEYBOARD_LL`·`WH_MOUSE_LL` callback은 입력을 차단하지 않고 bounded channel에 event를 전달한 뒤 즉시 `CallNextHookEx`를 호출한다.
- 기능을 켤 때 hidden `channel-ping-live`와 `channel-ping-overlay` WebView를 각각 준비한 뒤 input hook을 설치한다. 따라서 최초 Windows 키 뒤 WebView 생성으로 event 처리가 막히지 않으며, 기능을 끄면 hook과 두 hidden WebView를 모두 제거한다.
- 검증된 게임이 전경이고 실제 채널 연결이 있으면 client rect·DPI로 계산한 상단 중앙 520×30 live window에 최근 1분 TCP 추정 표본 `최대`와 값이 1 이상인 최근 1분 `재전송·시간초과·중복·혼잡` 발생량만 표시한다. 트래픽량에 따른 표본 갱신 특성을 실제 체감 개선으로 오해하지 않도록 순간값·평균값은 표시하지 않는다. 색상은 최대값 `0~40`, `41~70`, `71~100`, `101+ms`를 녹색·노란색·주황색·빨간색으로 나눈다. 모든 숫자는 `k/m/b` 고정 길이 축약을 적용하고 visible 배경은 `max-content`라 실제 문구와 최소 padding만 감싼다. foreground 판정은 100ms, TCP EStats 갱신은 1초 주기로 분리하며 Windows 키 전체표 중에는 지정된 Windows shell foreground만 예외로 허용한다. 비전경 숨김은 cached state뿐 아니라 `IsWindowVisible`로 실제 HWND를 재확인하고 `ShowWindowAsync(SW_HIDE)`를 반복 적용한다.
- 좌·우 Windows 키 down callback이 캡처한 같은 게임 PID로 실제 연결을 재검증하고 별도 584×400 overlay window를 중앙에 함께 표시한다. 두 창은 window ID·visible state·bounds·hide/destroy 수명주기를 공유하지 않는다.
- expanded 표시 trigger 당시 이미 눌린 입력만 release까지 무시한다. 그 밖의 keyboard·mouse down edge는 expanded만 즉시 숨긴다. live window는 게임 전경일 때 유지하고, expanded가 표시되는 동안에는 Windows shell focus 전환에도 같은 게임 위치에 유지하되 expanded 종료 후 게임이 전경이 아니면 숨긴다. 두 창 모두 focusable=false, always-on-top, skip-taskbar, transparent, ignore-cursor-events이며 `showInactive`로 표시한다.
- bounded input channel이 포화되면 event를 성공으로 간주하지 않고 fail-safe로 overlay를 숨긴다. backlog를 폐기한 뒤 최소 100ms의 input quiet 구간이 확인될 때까지 새 표시 trigger를 받지 않는다.
- 두 preload는 `channel-ping:get-status` 읽기 하나만 노출한다. 설정 쓰기와 창 command는 main renderer의 application channel과 backend service만 수행한다.
- package fallback source는 저장소 root `channel.csv` 하나이며 `scripts/package-dioxus.mjs`가 두 HTML·두 preload와 함께 복사한다. Electron 0.3.x 호환 host는 preload 요청을 명시적으로 거부하고 두 번째 writer나 기능 구현이 되지 않는다.

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
