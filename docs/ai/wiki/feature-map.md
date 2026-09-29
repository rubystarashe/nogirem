# 제품 기능 지도

- record_id: `FEATURE-MAP-NOGIREM-001`
- owner: `nogirem maintainer`
- revision: `2`
- updated_at: `2026-09-29T18:16:00Z`
- updated_by: `cursor-agent-92251f36`
- source_reviewed_at: `2026-09-29T18:09:00Z`
- source_reviewed_by: `cursor-agent-92251f36`
- source_review_target: `3cb35d86bf8967a86303b9440754445d9d338443, v0.4.2 replacement assets`
- source_review_evidence: `desktop/backend/src/network.rs`, `network_manager.rs`, `update_install.rs`, `service.rs`, NSIS와 PowerShell 종료 경로
- behavior_verified_at: `2026-09-29T18:15:00Z`
- behavior_verified_by: `cursor-agent-92251f36`
- behavior_verification_target: `3cb35d86bf8967a86303b9440754445d9d338443 기반 v0.4.2 replacement assets`
- behavior_verification_environment: `Windows 10.0.26200 x64, Node 테스트와 Rust test/check`
- behavior_verification_evidence: `npm test 198/198, backend Rust 57 executed with 2 declared ignores, cargo check, independent review APPROVED, public manifest Ed25519 and full asset download hashes`
- freshness_status: `CURRENT`
- freshness_reason: `0.4.2 대치 자산의 updater 무결성 소스와 공개 자산을 다시 검증함`
- known_gaps: `실제 외부 장치의 High label 종단간 update는 미실행이며 이미 실패 중인 0.4.1 helper는 새 payload 실행 전에 중단돼 0.4.2 수동 설치가 한 번 필요함`
- feature_impact: `FEAT-NOGIREM-NETWORK-FASTPING`, `FEAT-NOGIREM-UPDATE-LIFECYCLE`
- feature_map: `updated — 두 기능의 정본 identity와 동작·증거를 신규 지정함`
- architecture_impact: `네트워크 설정 writer와 updater·installer·helper 수명주기 경계`
- architecture_contract: `updated — agent-friendly-architecture.md와 상호 연결함`

## ID 할당

- 새 ID는 저장소 이름을 포함한 `FEAT-NOGIREM-<CAPABILITY>` 형식으로 maintainer가 예약한다.
- 공개된 ID는 이름이 바뀌어도 재사용하거나 번호를 바꾸지 않는다. 분할·병합·제거 시 기존 identity를 tombstone으로 유지한다.

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
- compatibility: 0.4.1 updater는 별도 installer 인자 없이도 검증된 부모 `updater.exe`로 인식한다. 새 updater는 `/NOGIREMUPDATE=<pid>`를 추가 전달한다.
- limits_exclusions: OS 종료·전원 손실 전체를 transaction으로 만들지는 않는다. 관련 backup이 없는 오래된 설치는 수동 복구가 필요할 수 있다.
- scenarios: `UPD-001 설치형 성공`, `UPD-002 건강 확인 실패 rollback`, `UPD-003 포터블 원자 교체`, `UPD-004 직접 제거 중 stale helper`, `UPD-005 늦게 생성된 프로세스`, `UPD-006 0.4.1 부모 updater 호환`, `UPD-007 native mandatory-label 검증`
- evidence: `test/installer.test.mjs`, `test/portable-release.test.mjs`, Rust updater/update_install tests, `docs/ai/runs/20260929-release-042`
- observability: update 상태·진행률·error, 보호 cache bootstrap log, `result.json`, `healthy.json`.
- architecture: [`agent-friendly-architecture.md`](agent-friendly-architecture.md), [`portable-distribution.md`](portable-distribution.md).
- known_gaps: 실제 배포 자산으로 이전 버전에서 업데이트하는 외부 장치 QA는 배포 후 사용자 확인이 필요하다.
