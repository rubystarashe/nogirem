# Agent-friendly architecture contract

- record_id: `ARCH-NOGIREM-001`
- owner: `nogirem maintainer`
- revision: `49`
- updated_at: `2026-10-21T14:10:00Z`
- updated_by: `cursor-agent-92251f36`
- source_reviewed_at: `2026-10-21T14:10:00Z`
- source_reviewed_by: `cursor-agent-92251f36`
- source_review_target: `e90af1a4304c8bc1201e5a7e5bbcbbd70a6419ed`
- source_review_evidence: `GLPIEx legacy/current cache relationship layout와 bounds SELF_REVIEW`
- behavior_verified_at: `2026-10-21T14:10:00Z`
- behavior_verified_by: `cursor-agent-92251f36`
- behavior_verification_target: `source e90af1a4304c8bc1201e5a7e5bbcbbd70a6419ed`
- behavior_verification_environment: `Windows 10.0.26200 x64, raw legacy GLPIEx fixture와 locked Rust/Node checks`
- behavior_verification_evidence: `GroupCount=0 단일 mask와 malformed multi-mask 회귀 PASS; backend 133 PASS·3 ignored와 parity 2/2, desktop 8/8·check, Node 215/215`
- freshness_status: `CURRENT`
- freshness_reason: `committed source의 Windows 10 cache topology compatibility와 검증 범위를 반영`
- known_gaps: `제보 i5-9400F Windows 10 실제 실행, package·push·배포는 NOT_RUN`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — Windows 10 CPU 분할 지원 복구`
- feature_map: `updated — canonical revision 45, frame revision 21`
- architecture_impact: `GLPIEx cache topology legacy/current 구조 호환`
- architecture_contract: `updated — revision 49`

## 계층과 확장 경로

1. `desktop/src`: Dioxus desktop host와 WebView 수명주기. 제품 설정을 직접 쓰지 않고 native service IPC를 사용한다.
2. `desktop/backend/src/service.rs`: renderer IPC 등록과 제품 service 조정. 무거운 기능 구현은 소유 module에 둔다.
3. `desktop/backend/src/<capability>.rs`: 기능별 authoritative logic과 Windows adapter 호출.
4. `desktop/backend/src/powershell`: PowerShell이 필요한 좁은 Windows adapter. 사용자 입력을 문자열로 직접 삽입하지 않는다.
5. `desktop/installer.nsi`, `desktop/portable.nsi`: 배포 bootstrap과 파일 설치·캐시 준비만 담당한다.
6. `native/installer-cleanup`: 설치·제거 중 소유 process와 정본 시작 작업만 정리하는 Rust/Win32 adapter다.
7. `scripts/package-dioxus.mjs`, `sign-update.mjs`: 재현 가능한 payload 조립과 매니페스트 서명 경로다.
8. `scripts/release-assets.mjs`, `prepare-release.mjs`: 서명된 원본을 자동 업데이트 호환 이름과 사용자 권장 alias로 조립하고 Electron feed·sha512·blockmap을 검증한다.
9. `src`, `service`: 0.3.x Electron 전환 호환 경로이며 Rust 신기능의 정본이 아니다.

새 제품 기능은 capability module과 제한된 service 등록점을 사용한다. `service.rs`, NSIS root, update root에 기능별 분기를 늘릴 때는 등록·수명주기 조정만 두고 실제 동작은 소유 module이나 script에 둔다.

## 의존성 방향과 금지 경계

- renderer → preload → desktop RPC → backend service → capability module → Windows adapter 순서만 허용한다.
- backend module은 renderer DOM이나 WebView animation 상태를 제품 준비의 authoritative source로 사용하지 않는다.
- 설치·제거는 이미지 이름 전역 kill을 사용하지 않는다. 정규화된 설치 경로, 보호 updater 경로, caller·parent PID 검증 없이 프로세스를 종료해서는 안 된다.
- network writer는 조회한 adapter GUID 이외의 registry interface 경로를 쓰지 않는다.
- 포터블 배포 EXE를 실행 중 직접 덮어쓰지 않는다. 보호 staging, digest, backup과 launcher 종료 경로를 사용한다.
- 사용자 설정·녹화 파일은 설치 manifest와 제거 manifest에 포함하지 않는다.
- Electron 호환 코드는 새 Rust backend authoritative state의 두 번째 writer가 될 수 없다.
- 그래픽 경로는 NVIDIA를 먼저 확인한다. NVIDIA가 감지되면 NVAPI 프로그램 profile을 authoritative target으로 사용하고 AMD WMI·ADLX helper를 호출하지 않는다.
- NVIDIA가 없는 AMD 시스템의 시작·새로고침은 `Win32_VideoController`의 `VEN_1002` 장치 식별만 허용하며 ADLX를 초기화하지 않는다. 명시 적용에서만 packaged `radeon-helper.exe`를 호출한다.
- Radeon helper는 호환 드라이버의 ADLX `I3DSettings`만 사용하고 수직 동기화·Enhanced Sync·Anti-Lag·Chill만 처리한다. GPU tuning·성능 monitoring API와 `InitializeWithIncompatibleDriver`는 금지하며 결과의 `tuningAccessed:false` 계약이 없으면 Rust·Electron 호출자가 실패로 처리한다.
- Radeon 명시 적용은 모든 대상 GPU의 네 3D 설정을 먼저 snapshot한다. 항목 실패 시 snapshot 복원을 시도하고 최종 목표 상태와 복원 오류를 반환하며, helper reason이 있으면 caller는 게임 VSync를 변경하지 않는다.
- UI runtime polling은 한 helper 요청 실패로 영구 종료하지 않는다. 실패한 기능의 `running`·`gameActive`를 false로 보정하고 다음 주기에 재조회한다.
- affinity 실행 상태는 상태 파일 timestamp만으로 판정하지 않고 해당 `helperPid`의 생존과 `helperStartedAt`이 현재 프로세스 시작 시각과 일치하는지 확인한다. helper lock도 PID와 process 시작 시각을 함께 기록하며 살아 있는 exact owner의 lock을 status 부재·staleness만으로 삭제하지 않는다. lock drop은 현재 record가 자신의 exact identity일 때만 파일을 삭제한다. coordinator는 시작 전 종료의 stderr를 직접 읽고, 시작 성공 시 같은 pipe reader를 bounded `helper-stderr.log` capture thread로 이전한다. 실행 중 stderr reader를 닫아 worker가 broken pipe로 종료되게 해서는 안 된다. reset은 current runtime ledger와 durable applied marker를 process identity별로 합쳐 복구한 뒤, 과거 결함으로 기록되지 않은 same-session process 중 현재 affinity가 이 topology의 exact app-generated mask와 같은 항목만 full system mask로 되돌린다.
- CPU topology와 mask 계산은 `topology.rs`만 소유한다. CPU Set의 `EfficiencyClass`로 P/E 소속을 판별하고 `SchedulingClass`는 같은 P-core 후보의 우선 선택에만 사용한다. 이를 GLPIEx physical core·SMT mask·L3 cache·processor group과 `(group, logical processor)`로 결합하고, 같은 L3 domain을 우선하되 cache 크기를 성능 등급으로 사용하지 않는다.
- 단일 `u64` process affinity로 표현할 수 없는 복수 processor group은 CPU 분할을 fail-safe로 비활성화한다. 물리 코어 수만으로 비활성화하지 않으며 단일 P-core 비하이브리드처럼 game/background 역할 mask를 만들 수 없는 경우에만 사유를 전달한다. logical-half fallback이나 잘린 mask를 적용해서는 안 된다.
- GLPIEx `RelationCache`는 Windows 10의 `GroupCount=0`·단일 `GroupMask`와 현재 Windows의 `GroupCount>=1`·`GroupMasks`를 모두 지원한다. 0은 legacy affinity 한 개로만 해석하고, 1 이상은 선언된 모든 mask가 record bounds 안에 있을 때만 사용한다. core·group relationship의 0 count와 잘린 배열은 계속 거부한다.
- P-core 개수 mask와 mode별 자동값은 `topology.rs`가 소유하고 affinity 적용 여부는 `affinity.rs`가 결정한다. focus는 balanced보다 최대 두 P-core를 더 선택하며 balanced/safe는 기존 자동값을 사용한다. focus/balanced는 game mask를 적용하고 safe는 게임을 고정하지 않는다. hybrid all-P에서는 background가 E-core-only가 되지 않도록 한 물리 P-core mask의 중첩을 허용한다. CPU 재정렬 capability는 현재 game mask와 겹치지 않는 동일 물리 P-core 수의 alternate mask가 존재할 때만 true이며 UI는 이 값을 우회해 실행 요청을 보내서는 안 된다.
- 외부 process mutation은 `process.rs`의 같은 handle에서 생성 시각, affinity, priority를 확인한 뒤 수행한다. 열거 뒤 PID만 다시 열어 검증·변경을 분리하는 경로를 추가해서는 안 된다.
- 터보키는 0.4.7부터 제품 runtime에서 시작하지 않는다. UI 진입점과 설정·다운로드 IPC는 실행 경로에 도달할 수 없어야 하며, 보존된 helper 구현을 다른 기능에서 호출해서는 안 된다. package와 release assembly는 helper binary나 standalone 약관 파일을 배포하지 않는다.
- DXVK는 확인되지 않은 게임 경로나 임의 URL에 파일을 쓰지 않는다. manager가 실제 `Client.exe`와 게임 미실행 상태를 확인하고 capability module이 digest·x64 PE·최종 파일 hash를 검증해야 한다.
- 채널 핑은 사용자 URL·hostname·endpoint를 받지 않는다. 고정 HTTPS source의 제한 크기 CSV를 전부 검증한 뒤에만 cache와 runtime endpoint를 교체한다.
- 실시간 점수 창은 게임 프로세스에 주입하거나 Windows 키를 차단하지 않는다. 실제 실행 경로가 정본 게임 조건을 충족한 전경 PID가 소유하고 CSV endpoint와 일치하는 established TCP 연결만 조회하며 비활성·click-through 상태를 유지한다. 상단 창은 게임 전경 또는 전체표가 열린 Windows shell 전경에서만 표시하고 일반 앱 전경에서는 마지막 게임 bounds를 재사용하지 않는다.
- 게임 화면 pixel을 캡처해 UI·상태·아이콘을 인식하는 기능은 금지한다. GDI·DirectX·screenshot capture, template matching, OCR, 해상도별 화면 좌표 cache와 이를 사용하는 알림 UI를 새로 추가하거나 우회 경로로 복원하지 않는다. 채널 핑 창은 OS TCP 상태만 읽고 게임 화면 pixel을 처리하지 않으므로 이 금지 범위와 분리한다.

## durable data와 single writer

- network original snapshot: `network_manager.rs`만 생성·갱신하고 network restore 경로만 소비한다.
- update job state: `update_install.rs` helper가 `job.json/result.json`을 관리한다. service는 검증된 job의 `healthy.json`만 기록한다.
- `healthy.json` 준비 기준: backend RPC가 실행되고 main UI open 요청이 성공한 시점이다. worker, PowerShell 상태 조회, DXVK 조회, 시작 애니메이션 완료는 필수 조건이 아니다.
- 자동 시작 예약 작업 상태 조회는 최대 3회 재시도한다. read-only 조회가 모두 실패하면 `unknown` projection만 반환해 해당 UI 조작을 비활성화하며 앱 시작·CPU boost를 실패시키지 않는다. 사용자의 명시적 설정 변경과 사후 검증 실패는 계속 오류로 처리한다.
- portable cache marker: packager가 내부 payload digest를 만들고 launcher만 marker를 쓴다.
- update manifest: release signing script만 private key를 사용해 작성하며 앱은 embedded public key로 검증만 한다.
- installer uninstall registry·shortcut writer: NSIS 설치기 하나다.
- game path state: affinity helper만 `%APPDATA%\마비노기 렘 부스터\game\path.json`을 쓰며 `Environment`와 입력 기능이 읽는다. 터보키의 허용 판단에는 사용하지 않는다.
- CPU setting: backend `Boost` service만 `affinity/game-core-setting.json`을 정규화해 쓴다. 저장 설정이 없으면 `cpuMode=balanced`, game affinity ON, background priority 하향 OFF를 사용하며 명시된 `cpuMode=focus|balanced|safe`, optional custom `gameCoreCount`와 priority 값은 보존한다. advanced count와 safe toggle은 상대 값을 보존하고, main preset 선택만 custom count를 null로 바꿔 자동값을 복원한다. UI dropdown은 persisted `customized` flag가 아니라 effective mode/count와 preset defaults를 비교해 standard preset 또는 동적으로 추가한 `사용자화` option을 투영한다. backend는 현재 topology 결과에서 파생한 `canCpuReorder`를 같은 setting projection에 포함하고 UI는 false일 때 action을 비활성화하고 topology·safe-mode 이유를 표시한다.
- affinity original-state ledger: exact helper lock owner 하나만 `runtime-state.json`, `status.json`, `applied-marker.json`을 쓴다. 변경 전 원본·앱 적용 affinity와 optional priority를 process identity와 함께 durable하게 기록하며, 시작 coordinator는 status를 선삭제하지 않는다. reset은 runtime ledger와 marker의 union을 사용하고 현재 값이 앱 적용값과 같은 known entry만 원본으로 바꿔 외부 변경을 보존한다. untracked legacy exact-mask 항목은 synthetic entry를 marker에 먼저 기록한 뒤 full system mask로 복구한다. 접근 오류로 복원하지 못한 entry는 marker에 남긴다.
- removed turbo setting: `inputs.rs`가 시작 시 기존 설정의 키·간격과 설치 파일은 보존하고 `enabled=false`만 기록한다. Electron 전환 호환 경로도 같은 비활성화만 수행하며 helper update·launch를 호출하지 않는다.
- DXVK state와 DLL: `dxvk.rs` install/apply 경로만 `%APPDATA%\마비노기 렘 부스터\vulkan`의 versioned DLL·`current.json`과 검증된 게임 폴더의 `d3d9_dxvk.dll`을 쓴다.
- channel ping setting·cache: `channel_ping.rs`만 `%APPDATA%\마비노기 렘 부스터\channel-ping\setting.json`과 검증된 `channel.csv`를 쓴다. 동일 module이 연결 시험 표본과 실제 5-tuple별 `SmoothedRtt + RttVar` TCP 추정값을 계산하고, 최근 유효 counter보다 재전송·TCP 시간초과·수신 중복 ACK·혼잡 신호 중 하나라도 증가한 표본에는 `SmoothedRtt + 4×RttVar`를 적용한다. 최근 1분 추정 표본 평균·최대, 네 counter delta 표본과 최근 유효 counter 기준은 메모리에만 두며 overlay·UI는 status IPC로 읽는다. 추정 최대는 원시 패킷 RTT 최대나 게임 처리·입력·렌더링 지연을 뜻하지 않는다. rolling outcomes·실제 연결·window mode는 지속하지 않는다.
- removed game overlay setting: 현재 source에는 `%APPDATA%\마비노기 렘 부스터\overlay\settings.json` writer나 reader가 없다. 기존 사용자 파일은 자동 삭제하지 않으며 어떤 runtime 결정에도 사용하지 않는다.

## 실시간 helper 상태와 게임 경로

- affinity·memory worker는 `status.json`에 `helperPid`, `helperStartedAt`, `updatedAt`, `running`, `gameActive`를 함께 기록한다. backend는 시작 요청 이후의 최신 timestamp와 exact PID 시작 identity를 모두 만족할 때만 새 실행을 성공으로 인정하고, child가 먼저 종료되면 pipe로 보존한 stderr 원인을 사용자에게 전달한다. 시작 성공 후 pipe reader는 worker 수명 동안 bounded logger가 소유한다.
- 메인 화면의 Vulkan 관리 control 아래에서 focus/balanced/safe preset을 선택한다. focus/balanced는 게임을 계산된 P-core mask에 고정하고 safe는 Windows scheduler에 맡긴다. focus preset은 일반 background role의 priority 하향도 함께 켜며 balanced/safe는 끈다. 고급 기능에서 count·safe·priority를 독립 변경할 수 있고 표준 조합과 다르면 사용자화로 투영한다.
- `helperStartedAt`이 없는 신선한 0.4.2 status와 살아 있는 PID는 migration 대상으로만 인정한다. 새 helper가 lock을 경쟁하기 전에 기존 control 경로로 중단하고 동일 PID·시작 시각 instance의 종료를 확인한다.
- UI는 2초 runtime polling을 유지한다. 개별 affinity/memory 오류는 사용자에게 한 번 표시할 수 있지만 polling task 자체를 종료하거나 이전 `gameActive=true`를 유지하지 않는다.
- 게임이 실행 중이 아니어도 helper는 대기 상태일 수 있으며 사용자의 부스트 중단·복원 조작을 차단하지 않는다.
- 터보키 helper source와 과거의 명시적 게임 경로 검증은 호환 기록으로 보존하지만 0.4.7 제품에서는 실행되지 않는다.
- 시작 migration은 실행 중일 수 있는 이전 helper에 control 종료 요청을 기록한 뒤 저장된 `enabled`를 false로 바꾸며, 실패 시 helper 실행으로 fallback하지 않는다.

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
- 기능을 켤 때 hidden `channel-ping-live`와 `channel-ping-overlay` WebView를 각각 준비한 뒤 input hook을 설치한다. 따라서 최초 Windows 키 뒤 WebView 생성으로 event 처리가 막히지 않으며, 기능을 끄면 hook과 두 hidden WebView를 모두 제거한다. 메인 창을 트레이로 숨기거나 다시 열 때 일반 보조 창은 정리하지만 두 channel monitor WebView는 제거하지 않으며, main renderer visibility와 무관하게 backend monitor thread가 표시 권한을 계속 판정한다.
- 검증된 게임이 전경이고 실제 채널 연결이 있으면 client rect·DPI로 계산한 상단 중앙 520×30 logical live window에 최근 1분 TCP 추정 표본 `최대`와 값이 1 이상인 최근 1분 `재전송·시간초과·중복·혼잡` 발생량만 표시한다. 트래픽량에 따른 표본 갱신 특성을 실제 체감 개선으로 오해하지 않도록 순간값·평균값은 표시하지 않는다. 색상은 최대값 `0~40`, `41~70`, `71~100`, `101+ms`를 녹색·노란색·주황색·빨간색으로 나눈다. 모든 숫자는 `k/m/b` 고정 길이 축약을 적용하고 visible 배경은 `max-content`라 실제 문구와 최소 padding만 감싼다. foreground 판정은 100ms, TCP EStats 갱신은 1초 주기로 분리하며 Windows 키 전체표 중에는 지정된 Windows shell foreground만 예외로 허용한다. 비전경 숨김은 cached state뿐 아니라 `IsWindowVisible`로 실제 HWND를 재확인하고 `ShowWindowAsync(SW_HIDE)`를 반복 적용한다.
- channel monitor 창의 logical 크기는 게임 HWND의 `GetDpiForWindow` 결과에 의존하지 않는다. DPI-unaware 게임은 고배율 모니터에서도 96 DPI를 반환할 수 있으므로 `MonitorFromWindow`와 `GetDpiForMonitor(MDT_EFFECTIVE_DPI)`로 대상 모니터 배율을 구해 584×400 전체표와 520×30 live 창을 physicalBounds로 환산한다. 모니터 DPI 조회가 실패할 때만 `GetDpiForWindow`를 fallback으로 사용하며 client rect·창 bounds·상단 offset은 physical 좌표 단위를 유지한다.
- 좌·우 Windows 키 down callback이 캡처한 같은 게임 PID로 실제 연결을 재검증하고 별도 584×400 overlay window를 중앙에 함께 표시한다. 두 창은 window ID·visible state·bounds·hide/destroy 수명주기를 공유하지 않는다.
- expanded 표시 trigger 당시 이미 눌린 입력만 release까지 무시한다. 그 밖의 keyboard·mouse down edge는 expanded만 즉시 숨긴다. live window는 게임 전경일 때 유지하고, expanded가 표시되는 동안에는 Windows shell focus 전환에도 같은 게임 위치에 유지하되 expanded 종료 후 게임이 전경이 아니면 숨긴다. 두 창 모두 focusable=false, always-on-top, skip-taskbar, transparent, ignore-cursor-events이며 `showInactive`로 표시한다.
- bounded input channel이 포화되면 event를 성공으로 간주하지 않고 fail-safe로 overlay를 숨긴다. backlog를 폐기한 뒤 최소 100ms의 input quiet 구간이 확인될 때까지 새 표시 trigger를 받지 않는다.
- 두 preload는 `channel-ping:get-status` 읽기 하나만 노출한다. 설정 쓰기와 창 command는 main renderer의 application channel과 backend service만 수행한다.
- package fallback source는 저장소 root `channel.csv` 하나이며 `scripts/package-dioxus.mjs`가 두 HTML·두 preload와 함께 복사한다. Electron 0.3.x 호환 host는 preload 요청을 명시적으로 거부하고 두 번째 writer나 기능 구현이 되지 않는다.

## 게임 화면 이미지 인식 금지

- `FEAT-NOGIREM-GAME-OVERLAY`는 `REMOVED`다. `game_overlay.rs`, 전용 settings·preview·alert·configuration HTML/preload, service monitor·IPC, native auxiliary allowlist, package CSV·template 입력을 지원 경로로 두지 않는다.
- 제품 기능은 게임 화면 pixel을 캡처·저장·분석하거나 OCR·template matching으로 상태를 추론해서는 안 된다. 기능명 변경, helper 분리, background service, 사용자 좌표 지정은 이 금지를 우회하지 못한다.
- 새 변경은 `game_overlay`, `overlay-settings`, `debuff-alert`, `debuff-region-overlay`, `overlay-configuration`, `debuff.csv`, package `data/conditions` 의존성을 추가하지 않아야 한다.
- `overlay_geometry.rs`와 `channel-ping-live`·`channel-ping-overlay`는 게임 화면 인식 코드가 아니다. 이들은 OS window geometry와 TCP 상태 표시를 위해 유지하며 화면 pixel capture 경로에 의존해서는 안 된다.
- 과거 run·review 문서는 immutable history로 유지하지만 현재 기능 근거로 사용할 수 없다. 재도입은 일반 exception으로 승인할 수 없으며 제품 정책 변경과 별도 architecture review가 필요하다.

## update·설치 transaction과 호환성

- manifest 서명과 payload SHA-256 검증 전에 설치를 시작하지 않는다.
- updater 작업은 migration mutex로 직렬화하고 보호 cache의 reparse와 Win32 mandatory-label ACL을 검증한다. `update.json`, payload, helper, `job.json`, `result.json`, 설치 승인 `commit`은 디렉터리 상속에만 의존하지 않고 생성 직후 파일별 High Integrity를 적용·재검증한다. PowerShell SDDL 문자열 표현은 신뢰 판정에 사용하지 않으며 label authority, NO_WRITE_UP 정책과 High/System RID를 모두 요구한다.
- 설치형은 파일·설정 backup 후 설치하고 건강 확인 실패 시 복원한다. 포터블은 동일 볼륨 원자 교체와 backup digest를 사용한다.
- 새 updater는 installer에 자신의 PID를 전달한다. NSIS는 canonical positive decimal만 helper에 고정 인자로 전달한다. 이전 0.4.1 updater는 인자가 없으므로 native helper가 경로와 생성 시각으로 검증한 parent `updater.exe` 하나를 자동 보존한다.
- 직접 설치·제거는 보호 updater 경로의 stale helper를 종료한다. 설치 폴더 프로세스가 정상 종료 중 다시 생길 수 있으므로 안정 구간까지 반복 조회한다.
- 서명 update helper는 기록된 app·backend PID 종료와 worker flush를 완료한 뒤 NSIS를 실행한다. native cleanup helper는 전달된 PID가 보호 cache의 정확한 parent `updater.exe` identity인지 검증한 경우 설치 폴더 프로세스를 다시 탐색하지 않는다. 직접 설치·제거와 인자 없는 0.4.1 호환 경로만 반복 종료 탐색을 소유한다.
- 신규 설치 대상에 기존 파일이 없으면 cleanup helper를 실행하지 않는다. 기존 설치 업데이트와 제거에서는 package가 현재 source로 빌드해 embedded한 `installer-cleanup.exe`만 실행하며 end-user PowerShell·`.ps1`·execution policy에 의존하지 않는다.
- helper는 설치 폴더와 보호 updater 폴더의 lexical/canonical UTF-16 경계를 사용한다. 종료 직전 같은 process handle에서 실행 경로와 생성 시각을 재검증하고 API·권한 실패를 fail-closed한다. PID가 재사용됐으면 새 process를 종료하지 않는다.
- 루트의 정본 `Mabinogi Rem Booster Startup` 작업 XML이 단일 `Exec` action으로 정확한 설치 실행 파일과 `--startup-tray`를 사용할 때만 `schtasks.exe`로 해제를 시도한다. 예약 작업 해제 실패는 실제 앱 process 종료 실패로 오인하거나 제거를 차단하지 않는다. 실제 process·updater 종료 실패만 fail-closed하며 NSIS는 encoding에 의존하지 않는 고정 한국어 오류를 표시한다.
- 이전 installer가 소유한 제거 기능·source-only asset은 versioned exact cleanup 목록으로만 지운다. 설치 폴더 전체, wildcard, `/r`, AppData와 알 수 없는 파일은 이 경로에서 삭제하지 않는다.
- 설치형·포터블 outer PE는 제품명, 설명, file/product version, internal/original filename과 copyright resource를 포함한다. 이는 Authenticode를 대체하지 않으며 update manifest의 Ed25519 서명도 Windows publisher 신뢰를 제공하지 않는다.
- 배포 압축은 scanner가 개별 payload 경계를 분석하기 쉬운 non-solid LZMA를 사용한다. package는 runtime·utility recorder 작업에 동일한 `native/recorder-helper/bin/recorder-helper.exe` 하나만 포함하며 `tools-bin` 복사본을 만들지 않는다.
- 설치형은 WebView2 bootstrapper를 payload에 포함하거나 실행하지 않는다. 시스템 registry에서 Microsoft Edge WebView2 Runtime을 확인하고 없으면 Microsoft 공식 HTTPS 다운로드 페이지를 열지 묻고, 설치 파일이나 registry를 변경하기 전에 중단한다.
- 설치형은 decimal 10MB 제한을 계속 fail-closed한다. signed 0.4.8 후보는 non-solid LZMA와 128kbps 시작 음악으로 `8,014,317 bytes`이며, 포터블 후보는 `7,789,628 bytes`다.
- retry는 새 attempt identity와 현재 target을 사용하며 이전 attempt의 늦은 callback이 현재 상태를 덮지 못한다.

## API·schema와 생성물

- renderer IPC channel은 native contract test에서 등록 목록을 검증한다.
- `update.json`과 `portable-update.json`은 분리된 서명 envelope이며 filename, version, size, SHA-256을 포함한다.
- 배포된 Rust verifier가 요구하는 `nogirem-dioxus-*`는 manifest용 canonical asset이며 짧은 이름은 byte-identical 수동 다운로드 alias다. Electron legacy 이름은 `latest.yml`의 path·두 sha512·필수 blockmap과 함께 변경한다.
- 같은 버전 hotfix 대치는 실제로 변경된 Rust executable·alias를 먼저 교체하고 signed manifests를 마지막에 교체한다. byte-identical Electron migration assets는 asset ID와 다운로드 연속성을 위해 재업로드하지 않으며 semver 자동 재수신 불가 가능성을 수동 재설치 안내로 공개한다.
- generated output: Rust web build, NSIS uninstall manifest, 설치형·포터블 EXE, signed manifests. authoritative input은 source, Cargo/npm lock, app assets, NSIS와 signing config다. 제거된 turbo helper·standalone terms는 generated output이나 release asset 목록에 포함하지 않는다.
- 생성 명령: `npm run package:win`. 생성된 `release/`과 `target/` 산출물은 source commit에 포함하지 않는다.

## enforcement

- `npm test`: renderer/preload 계약, 현재 source로 빌드한 native 설치·제거 실프로세스, 포터블·서명 정적 계약과 outer PE resource 입력.
- `cargo test --locked --manifest-path desktop/backend/Cargo.toml`: backend 기능·update 보안 경계.
- `cargo check --locked --manifest-path desktop/Cargo.toml`: desktop/backend compile 경계.
- package script: native service contract, Node/Electron payload 부재, single recorder helper, external WebView2 prerequisite, NSIS warning-as-error, 파일 크기, signing key/public key 일치.
- CI gap: dependency direction과 process termination 경로를 전용 architecture linter로 검사하지 않는다. 관련 정적 테스트와 독립 리뷰가 현재 gate다.

## 예외

- active exceptions: `none`
- 새 예외는 stable ID, owner, 승인, 정확한 파일·edge·만료 또는 제거 조건, 보완 통제와 feature/architecture impact를 기록하기 전에는 적용할 수 없다.
