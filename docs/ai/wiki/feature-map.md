# 제품 기능 지도

- record_id: `FEATURE-MAP-NOGIREM-001`
- owner: `nogirem maintainer`
- revision: `11`
- updated_at: `2026-10-05T07:07:00Z`
- updated_by: `cursor-agent-92251f36`
- source_reviewed_at: `2026-10-05T07:07:00Z`
- source_reviewed_by: `cursor-agent-92251f36`
- source_review_target: `source commit 193a0632776b5d6dd740571edad96a5c0b45b34c, base 5282e4bdff91602f91dae7630ccead12dae91106`
- source_review_evidence: `channel_ping.rs, service.rs, service_windows.rs, ui.rs, overlay HTML·tests SELF_REVIEW`
- behavior_verified_at: `2026-10-05T07:07:00Z`
- behavior_verified_by: `cursor-agent-92251f36`
- behavior_verification_target: `source commit 193a0632776b5d6dd740571edad96a5c0b45b34c`
- behavior_verification_environment: `Windows 10.0.26200 x64, Rust unit/integration·Node contract·desktop compile`
- behavior_verification_evidence: `channel ping backend 21/21, overlay interface 3/3, desktop build, diff check, IDE diagnostics`
- freshness_status: `CURRENT`
- freshness_reason: `0.4.5 실시간 점수 확인 source 설계와 현재 미검증 상태를 구분해 반영`
- known_gaps: `최신 평균·최대 RTT 표시 잘림, shell foreground 제한, 채널 전환 초기화, 혼합 DPI·package·배포는 NOT_RUN`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — 실제 접속 채널 상시 표시, 안정성 판정, 분산 측정, 기능명 변경`
- feature_map: `updated — 신규 기능 identity·흐름·writer·시나리오·검증 범위를 추가함`
- architecture_impact: `채널 CSV/cache writer, TCP worker, foreground input adapter, click-through auxiliary window`
- architecture_contract: `updated — agent-friendly-architecture.md에 채널 핑 기능 경계와 금지 규칙을 추가함`

## ID 할당

- 새 ID는 저장소 이름을 포함한 `FEAT-NOGIREM-<CAPABILITY>` 형식으로 maintainer가 예약한다.
- 공개된 ID는 이름이 바뀌어도 재사용하거나 번호를 바꾸지 않는다. 분할·병합·제거 시 기존 identity를 tombstone으로 유지한다.

## FEAT-NOGIREM-CHANNEL-PING

- canonical_name: `마비노기 실시간 점수 확인`
- aliases: `채널별 핑`, `채널 핑`, `핑 오버레이`
- lifecycle: `ACTIVE`
- lifecycle_reason: `GitHub v0.4.4 정식 Release로 설치형·포터블과 signed update manifest를 공개함`
- owners: `product=nogirem maintainer`, `technical=channel ping backend owner`
- purpose_and_scope: 고급 기능에서 켜면 공개 채널 endpoint의 초기 3표본을 약 30초에 순차 수집하고 이후 60초 주기로 분산 측정해 최근 20회 성공·실패의 변동 점수를 메모리에 유지한다. 검증된 마비노기 전경 TCP 연결이 CSV endpoint와 일치하면 별도 상단 창에 현재 채널·현재 Windows `SmoothedRtt`·최근 3분 표본 평균·표본 최대·재전송·상태를 표시하고, 좌·우 Windows 키로 독립된 중앙 전체 채널표를 함께 펼친다.
- exclusions: ICMP 왕복시간, 서버 내부 처리시간, 직접 패킷 손실 측정, 임의 endpoint·URL 입력, Windows 키 차단, 게임 프로세스 주입은 지원하지 않는다.
- personas_permissions_accessibility_entry: Windows 사용자가 고급 기능에서 선택적으로 켜고 끈다. 숫자와 `안정적`·`보통`·`불안정`·`연결 불가`·`확인 중` 문구를 함께 표시해 색상이나 네트워크 전문 용어에 의존하지 않는다.
- preconditions_dependencies: Windows, 실행 중인 Rust 앱, 검증 가능한 마비노기 `Client.exe`, 저수준 keyboard·mouse hook, 고정 GitHub raw HTTPS URL, WebView2, 공개 채널 TCP endpoint.
- normal_error_cancel_retry_flow: 앱 시작 시 원격 CSV를 제한 크기·UTF-8·header·연속 channel·IP·port 기준으로 검증하고 성공 시 cache에 원자 저장한다. 실패하면 마지막 유효 cache, 그다음 package CSV를 사용한다. endpoint 연결 실패가 있으면 시작 확인 이후 최대 1시간에 한 번 원격 파일을 다시 확인한다.
- concurrency_partial_failure_recovery: CSV 갱신 operation lock이 cache 교체를 직렬화하고 RwLock snapshot이 overlay reader와 측정 worker를 분리한다. 한 채널 실패가 다른 이력을 지우지 않고 endpoint가 바뀐 채널만 이전 표본을 폐기한다. 목록 최저 점수 동률은 같은 품질 색상으로 함께 강조한다.
- state_side_effects_limits: `%APPDATA%\마비노기 렘 부스터\channel-ping\setting.json`과 검증된 `channel.csv` cache만 지속한다. 채널 표본과 실제 연결의 최근 3분 `SmoothedRtt` 표본·창 mode는 메모리에만 있으며 앱 종료·기능 해제 시 폐기된다. 실제 연결 identity가 바뀌면 평균·표본 최대 이력을 초기화한다. 표본 최대는 원시 패킷 RTT 최대를 보장하지 않는다. 검증된 게임이 전경일 때만 1초마다 TCP table을 읽고 해당 connection의 Windows EStats 수집을 활성화·조회한다.
- modules_contracts: `desktop/src/ui.rs` → `service.rs` → `channel_ping.rs`; `service_windows.rs`와 `desktop/src/native.rs`가 `channel-ping-live`·`channel-ping-overlay`의 독립 window identity를 중계하고 두 preload는 같은 읽기 전용 status IPC 하나만 노출한다.
- durable_data_owner: `channel_ping.rs`가 setting과 CSV cache의 유일한 writer다. 패키지 `channel.csv`는 fallback source이며 packager만 배포 payload에 복사한다.
- security_privacy: 고정 HTTPS URL과 공개 IPv4 literal·port만 허용하며 사용자 endpoint나 인증값을 받지 않는다. 실행 경로가 정본 게임 조건을 충족한 전경 process instance가 소유하고 CSV endpoint와 정확히 일치하는 established 5-tuple만 처리한다. 게임 메모리·패킷을 읽거나 주입하지 않고 공개 endpoint 외 사용자 데이터를 전송하지 않는다.
- scenarios: `PING-001 startup 원격 갱신`, `PING-002 cache·package fallback`, `PING-003 초기 가속·분산 측정·20회 변동 점수`, `PING-004 실패 중 1시간 갱신`, `PING-005 게임 전경 compact 표시·비전경 숨김`, `PING-006 독립 Win 키 expanded·후속 down은 expanded만 숨김`, `PING-007 실제 PID·endpoint TCP EStats`, `PING-008 비활성·비전경 유휴`, `PING-009 두 HTML·preload package 입력`, `PING-010 점수 색상·최저 강조`, `PING-011 실제 연결 평균·최대·재전송과 identity 초기화`
- observability_evidence: 고급 기능 sync fallback 문구, overlay 측정 경과 시간·채널 상태, `channel_ping.rs` unit tests, `test/overlay-interface.test.mjs`, native IPC contract test.
- architecture: [`agent-friendly-architecture.md`](agent-friendly-architecture.md)의 채널 핑 network·input·auxiliary window 경계.
- record_id: `FEAT-NOGIREM-CHANNEL-PING`
- owner: `nogirem maintainer`
- revision: `6`
- updated_at: `2026-10-05T07:07:00Z`
- updated_by: `cursor-agent-92251f36`
- source_reviewed_at: `2026-10-05T07:07:00Z`
- source_reviewed_by: `cursor-agent-92251f36`
- source_review_target: `source commit 193a0632776b5d6dd740571edad96a5c0b45b34c, base 5282e4bdff91602f91dae7630ccead12dae91106`
- source_review_evidence: `run task source paths SELF_REVIEW`
- behavior_verified_at: `2026-10-05T07:07:00Z`
- behavior_verified_by: `cursor-agent-92251f36`
- behavior_verification_target: `source commit 193a0632776b5d6dd740571edad96a5c0b45b34c`
- behavior_verification_environment: `Windows 10.0.26200 x64 automated source target`
- behavior_verification_evidence: `channel ping backend 21/21, overlay interface 3/3, desktop build, diff check, IDE diagnostics`
- freshness_status: `CURRENT`
- freshness_reason: `0.4.5 source 의미와 QA 대기 상태를 정확히 반영`
- known_gaps: `최신 실제 연결 평균·최대 UI와 비전경 숨김·DPI·설치·제거 종단간은 NOT_RUN`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — 실시간 점수 확인 개편`
- feature_map: `updated — 이 entry를 신규 canonical identity로 추가`
- architecture_impact: `channel ping capability·cache writer·Win32 input·auxiliary window 경계`
- architecture_contract: `updated — 관련 contract section과 연결`

## FEAT-NOGIREM-FRAME-BOOST

- canonical_name: `마비노기 실시간 프레임 부스트`
- aliases: `실시간 부스트`, `CPU affinity 최적화`
- lifecycle: `ACTIVE`
- lifecycle_reason: `Rust/Dioxus 앱이 게임 실행 상태에 따라 affinity·메모리 helper를 유지하고 사용자가 적용·중단할 수 있음`
- owners: `product=nogirem maintainer`, `technical=boost/affinity backend owner`
- purpose_and_scope: 관리자 권한 Windows 사용자의 마비노기 실행을 감지해 게임과 백그라운드 CPU 범위를 관리하고, 게임 미실행 중에는 대기 상태를 표시하며 언제든 중단·복원한다.
- exclusions: 하드웨어·드라이버 교착 복구, 타 프로그램의 별도 affinity writer 통제, 비정상 종료 전 임의 custom mask의 완전한 원상복구는 보장하지 않는다.
- personas_permissions_accessibility_entry: 일반 사용자가 메인 화면을 클릭해 적용·중단하며 관리자 권한이 필요하다. 상태는 문구로 표시하고 오류는 확인 가능한 alert로 제공한다.
- preconditions_dependencies: Windows CPU topology, `config.json`, affinity/memory helper, Win32 process query, `%APPDATA%\마비노기 렘 부스터` 상태 폴더.
- normal_error_cancel_retry_flow: helper 상태의 PID·시작 시각·생존을 함께 확인한다. 게임이 없으면 대기하지만 중단 버튼은 활성이다. runtime 조회 실패 시 화면 상태를 중단으로 보정하고 polling을 계속한다. `helperStartedAt`이 없는 살아 있는 0.4.2 helper는 control 명령으로 종료한 뒤 0.4.3 helper로 교체한다.
- concurrency_partial_failure_recovery: affinity·memory operation lock이 작업을 직렬화한다. 두 helper 중 하나가 실패하면 해당 runtime을 비실행으로 표시하며 사용자가 중단하거나 ensure 경로가 재시작한다.
- state_side_effects_limits: 프로세스 affinity, memory cleanup, 선택적 NIC RSS를 변경한다. `status.json`, `control.json`, `runtime-state.json`, `applied-marker.json`, `events.log`를 사용한다.
- modules_contracts: `desktop/src/ui.rs` → `service.rs` → `boost.rs` → `affinity_worker.rs`/`affinity.rs`. helper PID와 최신 timestamp가 실행 상태의 필수 조건이다.
- durable_data_owner: affinity helper가 affinity runtime과 `game/path.json`의 유일한 writer이며 backend/UI가 읽는다.
- security_privacy: 다른 `Client.exe` 오탐을 막기 위해 정본의 명시적 게임 폴더를 사용하며 진단 내 사용자 경로는 마스킹한다.
- scenarios: `BOOST-001 helper 종료 stale 상태 제거`, `BOOST-002 polling 오류 뒤 복구`, `BOOST-003 게임 미실행 중 중단`, `BOOST-004 0.4.2 helper 인계`, `BOOST-005 게임 시작·종료 affinity 적용·복원`
- observability_evidence: UI 상태 문구, `affinity/events.log`, diagnostics affinity snapshot, `boost::tests`, `test/application-reliability.test.mjs`
- architecture: [`agent-friendly-architecture.md`](agent-friendly-architecture.md)의 helper runtime·game path 경계.
- freshness_status: `CURRENT`
- known_gaps: `helper 강제 종료와 실제 게임 종료를 결합한 수동 UI QA는 NOT_RUN`

## FEAT-NOGIREM-TURBO-KEY

- canonical_name: `마비노기 전용 터보키`
- aliases: `터보 키`, `반복 입력 helper`
- lifecycle: `ACTIVE`
- lifecycle_reason: `사용자가 별도 helper를 내려받아 선택 키를 마비노기 전경 창에서만 반복 입력할 수 있음`
- owners: `product=nogirem maintainer`, `technical=input helper owner`
- purpose_and_scope: 선택한 비수정키를 1~30ms 간격으로 반복하며 정식·테스트 서버 `Client.exe`가 전경일 때만 입력한다.
- exclusions: 마비노기 외 창, 허용되지 않은 modifier/lock/Esc, background 입력, 자동 게임 플레이 판단은 지원하지 않는다.
- personas_permissions_accessibility_entry: 고급 기능에서 안내 동의 후 helper를 다운로드·설정한다. 실행·업데이트 오류를 텍스트로 표시한다.
- preconditions_dependencies: Windows x64, 저수준 keyboard hook, 서명 release의 SHA-256 자산, 정본에 명시된 게임 폴더.
- normal_error_cancel_retry_flow: 키 입력 시 전경 PID의 실제 경로를 확인하고 `Mabinogi`, `Mabinogi_Test`, `마비노기`, `Nexon` 폴더 또는 sibling `Mabinogi.exe`가 있는 경로만 허용한다. 포커스·modifier·부모 앱이 바뀌면 즉시 반복을 취소한다.
- concurrency_partial_failure_recovery: named mutex가 단일 helper를 보장한다. 앱은 설정 변경·업데이트 전에 이전 helper를 중단하고, 구버전 0.1.7은 동의를 보존한 채 0.1.8로 교체한다.
- state_side_effects_limits: 저수준 hook과 `SendInput`을 사용하며 설정·설치 manifest·status/control JSON만 지속한다.
- modules_contracts: `inputs.rs`가 실행 인자와 설치 상태를 소유하고 `native/turbo-key`가 입력 동작과 명시적 게임 폴더 판별을 소유한다.
- durable_data_owner: turbo 설정·manifest writer는 inputs installer 경로 하나이며 native helper는 실행 중 상태만 기록한다.
- security_privacy: 실행 파일 PE·digest·GitHub 자산을 검증하고, 전경 PID의 전체 경로가 명시 폴더 또는 launcher 조건을 충족할 때만 입력한다.
- scenarios: `TURBO-001 정식 서버`, `TURBO-002 테스트 서버`, `TURBO-003 다른 Client.exe 거부`, `TURBO-004 modifier·포커스 이탈 취소`
- observability_evidence: turbo status JSON, UI 실행 오류, helper 14개 unit test, `test/turbo-key.test.mjs`
- architecture: [`agent-friendly-architecture.md`](agent-friendly-architecture.md)의 선택 helper·명시 게임 폴더 경계.
- freshness_status: `CURRENT`
- known_gaps: `실제 테스트 서버 창에서 1ms~30ms 반복 입력 수동 확인은 NOT_RUN`

## FEAT-NOGIREM-DXVK

- canonical_name: `마비노기 Vulkan(DXVK) 설치·적용`
- aliases: `불칸`, `DXVK`
- lifecycle: `ACTIVE`
- lifecycle_reason: `Rust/Dioxus 앱이 검증된 DXVK 정식 릴리즈를 내려받아 게임의 D3D9 DLL로 적용하고 상태를 확인함`
- owners: `product=nogirem maintainer`, `technical=DXVK backend owner`
- purpose_and_scope: 사용자가 호환 DXVK 버전을 선택하면 GitHub 정식 자산의 SHA-256을 검증하고 x64 `d3d9.dll`을 전용 저장소에 보관한 뒤 확인된 마비노기 폴더의 `d3d9_dxvk.dll`로 적용한다.
- exclusions: Windows 보안 정책·백신 예외를 자동 변경하지 않으며 실행 중인 게임 DLL 강제 교체, 임의 다운로드 URL·사전 릴리즈·x86 DLL은 지원하지 않는다.
- personas_permissions_accessibility_entry: 고급 기능의 Vulkan 관리 화면에서 상태·호환 버전·오류를 텍스트로 확인하고 설치한다. 게임 폴더 쓰기 권한이 필요하다.
- preconditions_dependencies: 확인 가능한 `Client.exe` 경로, 종료된 게임, 호환 GPU driver, GitHub DXVK release, asset digest, NTFS/Windows 파일 API.
- normal_error_cancel_retry_flow: 게임 경로를 다운로드 전에 확인하고 자산·압축·DLL을 검증한다. 같은 폴더의 UUID 임시 파일을 동기화·재검증한 뒤 기존 DLL을 원자 교체하고 최종 hash를 확인한다. 경로 누락, 파일 사용, 권한 거부, 보안 제품의 임시·최종 DLL 삭제를 단계별 한국어 오류로 안내하며 사용자가 원인을 해소한 뒤 재시도한다.
- concurrency_partial_failure_recovery: manager lock이 앱 내 설치를 직렬화한다. 허용 게임 폴더·launcher와 모든 상위 reparse 경계를 검사하며 다운로드 뒤 게임 실행 여부와 canonical 대상 경로를 교체 직전에 다시 확인한다. 기존 DLL은 검증된 backup을 만들고 최종 교체·검증 뒤 정리하며, 교체 뒤 실패하면 backup 복구를 시도한다.
- state_side_effects_limits: `%APPDATA%\마비노기 렘 부스터\vulkan`의 versioned DLL·`current.json`과 게임 폴더의 `d3d9_dxvk.dll`을 쓴다. 설치된 게임 원본 `d3d9.dll`은 변경하지 않는다.
- modules_contracts: `service.rs` → `dxvk_manager.rs` → `dxvk.rs` → GitHub HTTP·Windows `SetFileInformationByHandle`. manager만 게임 경로·실행 상태를 조정하고 `dxvk.rs`가 digest·archive·file transaction을 소유한다.
- durable_data_owner: `dxvk.rs` install/apply 경로가 versioned DLL, `current.json`, `d3d9_dxvk.dll`의 유일한 앱 writer다.
- security_privacy: 고정 GitHub release prefix와 공개 SHA-256을 요구하고 leaf filename만 허용한다. 오류는 작업 단계를 노출하되 사용자 전체 경로를 메시지에 포함하지 않는다.
- scenarios: `DXVK-001 기존 DLL 원자 교체`, `DXVK-002 누락된 저장 폴더 생성`, `DXVK-003 게임 경로·reparse 선차단`, `DXVK-004 교체 직전 게임 실행·경로 변경 차단`, `DXVK-005 보안 격리·권한·잠금 오류 안내`, `DXVK-006 저장·적용 후 hash 검증`, `DXVK-007 실패 시 기존 DLL 보존·임시 파일 정리`
- observability_evidence: Vulkan 관리 UI 오류·상태, backend DXVK unit tests, `test/vulkan.test.mjs`, 진단의 Vulkan 상태 파일.
- architecture: [`agent-friendly-architecture.md`](agent-friendly-architecture.md)의 DXVK download·file transaction 경계.
- freshness_status: `CURRENT`
- known_gaps: 제보 환경의 백신 격리와 실제 게임 폴더 쓰기는 수동 재현하지 못했다.

## FEAT-NOGIREM-NETWORK-FASTPING

- canonical_name: `주 네트워크 패스트핑 최적화`
- aliases: `패스트핑`, `TcpAckFrequency/TCPNoDelay 최적화`
- lifecycle: `ACTIVE`
- lifecycle_reason: `설치형과 포터블 Rust 앱에서 지원하며 Electron 전환 경로도 호환 유지`
- owners: `product=nogirem maintainer`, `technical=network backend owner`
- purpose: 사용자가 현재 주 IPv4 물리 adapter에 TCP ACK 지연 완화 값을 적용·확인·복원한다.
- personas_and_entry_points: 관리자 권한 Windows 사용자가 메인 최적화 화면 또는 CLI 상태·적용 명령에서 사용한다.
- accessibility: 결과와 실패 원인을 텍스트로 표시하며 색상만으로 상태를 구분하지 않는다.
- preconditions: Windows, 관리자 권한은 쓰기 때만 필요, 연결된 IPv4 기본 경로, 호환 가능한 물리 adapter.
- normal_flow: 기본 경로의 interface GUID를 찾고 원래 DWORD를 저장한 뒤 두 값을 1로 기록·재조회하고 필요할 때 adapter를 재시작한다.
- error_retry_flow: 기본 경로 부재는 예외 스택이 아닌 `supported=false`, `disconnected=true`와 재연결 안내를 반환한다. 연결 후 사용자가 다시 시도한다.
- cancellation_concurrency: 네트워크 manager lock이 앱 내 동시 작업을 직렬화하고 적용 검증 실패나 연결성 악화 시 원래 값을 복원한다.
- partial_failure_recovery: 쓰기 후 값 검증 실패는 성공으로 표시하지 않으며 연결성 검사가 기준보다 악화되면 저장된 GUID의 원래 DWORD로 rollback한다.
- state_and_side_effects: `%APPDATA%\마비노기 렘 부스터\network\fast-ping-original.json`에 원본을 기록하고 `HKLM\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces\<GUID>`의 두 DWORD를 변경한다.
- dependencies_and_contracts: `Get-NetIPInterface`, `Get-NetRoute`, `Get-NetAdapter`, registry, Rust `network_manager`. GUID가 authoritative identity이며 index는 보조 값이다.
- durable_data: 원본 snapshot writer는 network manager 하나이며 앱만 읽고 복원한다. 사용자가 초기화하지 않는 한 유지된다.
- security_privacy: 임의 registry 경로를 입력받지 않고 조회한 GUID만 사용한다. 진단 ZIP은 IP를 마스킹한다.
- limits_exclusions: IPv6 전용, VPN·가상 adapter, 알려지지 않은 차단 필터에는 적용하지 않는다. 물리적 인터넷 단절은 앱이 복구하지 않는다.
- scenarios: `NET-FP-001 적용·검증`, `NET-FP-002 이미 적용됨`, `NET-FP-003 기본 경로 없음`, `NET-FP-004 연결성 악화 rollback`, `NET-FP-005 저장값 복원`
- evidence: `test/network.test.mjs`, `desktop/backend/src/network.rs` tests, 진단 report `55d16a69-0d32-4bfd-a118-51b03b89fab3`
- observability: UI network status·reason, diagnostics network snapshot, startup log의 네트워크 조회 완료.
- architecture: [`agent-friendly-architecture.md`](agent-friendly-architecture.md)의 네트워크 writer 경계.
- known_gaps: 실제 ISP 단절·복구 시나리오의 수동 UI 확인은 남아 있다.

## FEAT-NOGIREM-UPDATE-LIFECYCLE

- canonical_name: `서명 업데이트·설치·제거 수명주기`
- aliases: `자동 업데이트`, `설치형 업데이트`, `포터블 업데이트`
- lifecycle: `ACTIVE`
- lifecycle_reason: `0.4.1부터 설치형·포터블 서명 경로를 제공하며 0.4.2에서 종료·건강 확인 경계를 보강`
- owners: `product=nogirem maintainer`, `technical=release/update owner`
- purpose: 서명된 새 버전을 다운로드·검증·설치하고 실패 시 rollback하며 사용자가 앱을 잠금 없이 제거하도록 한다.
- personas_and_entry_points: 일반 사용자는 앱 업데이트 UI, Windows 설치기·제거기, 포터블 EXE로 진입한다. 배포 maintainer는 서명 패키징과 GitHub Release를 사용한다.
- accessibility: 진행률·오류·재시도·숨기기와 재열기 경로를 제공한다.
- preconditions: 관리자 권한, 신뢰된 Ed25519 manifest, 예상 SHA-256, 보호된 updater 작업 경로, 설치형 또는 검증된 포터블 원본.
- normal_flow: manifest 검증 → payload 검증 → 앱 종료 → 설치 또는 원자 교체 → backend가 main UI 열기 성공 후 `healthy.json` 기록 → 완료.
- error_retry_flow: 다운로드·설치·건강 확인 실패는 오류를 기록하고 설치형 파일·설정 또는 포터블 EXE를 검증된 backup으로 복원한다.
- cancellation_concurrency: migration mutex와 보호 작업 폴더가 동시 update를 막는다. 직접 설치·제거는 stale updater를 종료하지만 신뢰된 부모 updater 하나는 보존한다.
- partial_failure_recovery: helper 중단 포터블 작업은 launcher PID·경로·backup digest를 검증해 복원한다. 설치형은 설치 전 파일과 설정을 복사해 rollback한다.
- uninstall_flow: 시작 예약 작업을 먼저 해제하고 보호 updater 경로의 stale helper를 종료한 뒤 설치 폴더 프로세스를 매 반복 재탐색한다. 30초 정상 종료 후에도 남은 소유 프로세스만 제한 종료한다.
- state_and_side_effects: `%LOCALAPPDATA%\NogiremUpdater\updates\job-*` 작업 상태, 설치 파일, registry uninstall 항목, 바로가기, 시작 예약 작업을 관리한다.
- dependencies_and_contracts: Rust updater가 유일한 update 작업 writer이며 NSIS는 설치 파일 writer, stop script는 경로 제한 process terminator다. Electron 0.3.19는 Rust 전환 manifest로 연결된다.
- durable_data: 작업 `job.json/result.json/healthy.json`은 updater가 쓰고 service는 검증된 작업의 건강 확인만 쓴다. 사용자 설정과 녹화 파일은 제거·업데이트 대상이 아니다.
- security_privacy: manifest 서명·digest, reparse 방지, Win32 mandatory-label authority·NO_WRITE_UP·High/System RID, exact executable path와 parent PID 검증을 사용한다. 개인 데이터나 signing key를 자산에 포함하지 않는다.
- compatibility: 0.4.1 updater는 별도 installer 인자 없이도 검증된 부모 `updater.exe`로 인식한다. 배포된 0.4.x URL 검증을 위해 `nogirem-dioxus-*` 자산은 자동 업데이트 전용으로 유지하며 짧은 `nogirem-setup-*`, `nogirem-portable-*`는 동일 payload의 수동 다운로드 alias다. Electron은 `latest.yml`이 긴 legacy migration 이름을 가리킨다.
- limits_exclusions: OS 종료·전원 손실 전체를 transaction으로 만들지는 않는다. 관련 backup이 없는 오래된 설치는 수동 복구가 필요할 수 있다.
- scenarios: `UPD-001 설치형 성공`, `UPD-002 건강 확인 실패 rollback`, `UPD-003 포터블 원자 교체`, `UPD-004 직접 제거 중 stale helper`, `UPD-005 늦게 생성된 프로세스`, `UPD-006 0.4.1 부모 updater 호환`, `UPD-007 native mandatory-label 검증`
- evidence: `test/installer.test.mjs`, `test/portable-release.test.mjs`, Rust updater/update_install tests, `docs/ai/runs/20260929-release-042`
- observability: update 상태·진행률·error, 보호 cache bootstrap log, `result.json`, `healthy.json`.
- architecture: [`agent-friendly-architecture.md`](agent-friendly-architecture.md), [`portable-distribution.md`](portable-distribution.md).
- known_gaps: 실제 배포 자산으로 이전 버전에서 업데이트하는 외부 장치 QA는 배포 후 사용자 확인이 필요하다.
