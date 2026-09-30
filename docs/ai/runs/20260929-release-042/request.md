# 0.4.2 패스트핑·업데이트 수명주기 보강 요청

- record_id: `REQ-RELEASE-042-01`
- run_id: `20260929-release-042`
- checkpoint_id: `CP-RELEASE-042-01`
- owner: `cursor-coordinator-92251f36`
- created_at: `2026-09-29T13:29:00Z`
- target: `nogirem/main@caf15cd3bca07cd106a5688118bc2bd163b07540`

## 요청

- 진단 ZIP `nogirem-diagnostics-0.3.16-20260929-211325-55d16a69-0d32-4bfd-a118-51b03b89fab3 - 반그루.zip`의 패스트핑 실패를 분석하고 Rust 0.4.x의 동일 결함을 수정한다.
- 업데이트 건강 확인을 UI 애니메이션과 분리한다.
- PowerShell `exit 1` 정상 제어 흐름과 설치·제거 시 창 활성화 가능성을 제거한다.
- `%LOCALAPPDATA%\NogiremUpdater` helper 재실행과 설치 폴더 프로세스 재생성을 제거 과정에서 차단한다.
- 실제 종단간 회귀 테스트를 추가하고 검증이 통과하면 0.4.2를 서명·배포한다.

## 진단과 범위

- 진단 시점의 업데이트 오류는 `net::ERR_INTERNET_DISCONNECTED`였고 패스트핑 레지스트리 값 `TcpAckFrequency=1`, `TCPNoDelay=1`은 이미 적용돼 있었다.
- 캡처된 오류는 IPv4 기본 경로가 일시적으로 없을 때 `Get-NetRoute`가 `ErrorActionPreference=Stop`에 의해 전체 예외로 승격되는 결함이다. Electron과 Rust가 같은 쿼리 구조를 사용하므로 둘 다 영향받는다.
- 포함: Electron 호환 경로, 현재 Rust 경로, 설치형·포터블 런처, updater helper, 제거 스크립트, 자동 회귀, 정본 문서, 0.4.2 GitHub 릴리스.
- 제외: 인터넷이 물리적으로 끊긴 상태에서 패스트핑을 강제 적용하는 동작, 운영체제 네트워크 복구, 0.3.x 신규 배포.

## 인수 기준과 권한

1. 기본 경로가 없으면 PowerShell 스택 대신 구조화된 연결 없음 상태를 반환한다.
2. Rust 앱은 메인 UI 요청이 성공한 백엔드 준비 시점에 `healthy.json`을 기록한다.
3. 정상적인 프로세스 없음 상태에 PowerShell 종료 코드 1을 사용하지 않는다.
4. 직접 설치·제거에서는 updater helper를 종료하고 새 설치 프로세스를 계속 재탐색한다. updater가 호출한 설치는 현재 helper를 보존한다.
5. 설치·제거 PowerShell은 콘솔 없는 실행 경로를 사용한다.
6. 관련 자동 테스트, Rust 빌드, 실서명 패키징과 공개 자산 검증이 통과한다.
7. 사용자가 0.4.2 배포를 명시적으로 승인했다. 로컬 커밋과 GitHub 릴리스 생성·자산 업로드는 허용되며 배포 외 운영 변경과 이력 재작성은 금지한다.

## 위험과 영향

- risk: `HIGH`
- rationale: 설치·제거, 프로세스 종료, 업데이트 롤백, 서명 자산과 공개 배포를 변경한다. 잘못되면 실행 파일 잠금, 불완전 제거, 롤백 또는 업데이트 불능이 발생할 수 있다.
- feature_impact: `FEAT-NOGIREM-NETWORK-FASTPING — 연결 단절 오류 처리`, `FEAT-NOGIREM-UPDATE-LIFECYCLE — 건강 확인·설치·제거·helper 종료·포터블 준비`
- feature_map: `blocked — 정본 feature map이 없으므로 이번 실행에서 생성·연결해야 함; owner cursor-coordinator-92251f36`
- architecture_impact: `업데이트 실행 권한 경계, updater 단일 작업 경로, installer/helper 프로세스 수명주기, 네트워크 설정 writer 경계`
- architecture_contract: `blocked — 정본 architecture contract가 없으므로 검증된 현재 경계와 이번 보강을 기록해야 함; owner cursor-coordinator-92251f36`

## CP-RELEASE-042-02 대치 요청

- received_at: `2026-09-29T17:51:00Z`
- original_request: `수정하고 대치재배포해`
- objective: 정상 High integrity 작업 파일을 PowerShell SDDL 문자열 누락으로 거부하는 0.4.2 updater를 수정하고 동일 GitHub v0.4.2 자산을 대치한다.
- acceptance: native label 검증 회귀, 전체 Node/Rust 검증, 독립 리뷰, Ed25519 패키징, 설치형·포터블·매니페스트 대치와 공개 실다운로드 검증.
- constraint: 이미 실패 중인 0.4.1 helper는 새 payload 실행 전에 종료되므로 서버 자산만으로 self-heal할 수 없다. 해당 사용자는 대치 0.4.2를 한 번 수동 설치해야 한다.
- authorization: 기존 v0.4.2 release asset 대치 허용. 기존 tag 재작성, main push, 사용자 장치 설치는 제외.
- risk: `HIGH — 업데이트 신뢰 경계와 공개 실행 파일 대치`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — 작업 파일 신뢰 오판 제거`
- feature_map: `updated — native label 시나리오와 배포 한계를 반영`
- architecture_impact: `mandatory integrity label 조회·정책 검증 경계`
- architecture_contract: `updated — PowerShell 문자열 대신 Win32 ACL 검증을 정본화`

## CP-RELEASE-042-03 파일명 전환 요청

- received_at: `2026-09-30T09:53:00Z`
- original_request: 기존 Electron 설치 파일은 긴 legacy 이름으로, Rust 설치형·포터블은 `dioxus`를 뺀 정식 이름으로 변경.
- objective: 수동 다운로드 사용자를 정식 Rust 설치형·포터블로 유도하고 Electron 전환본 오다운로드를 줄인다.
- compatibility decision: 배포된 0.4.x verifier가 `nogirem-dioxus-*` URL을 요구하므로 signed manifest와 호환 자산은 유지하고 짧은 이름을 byte-identical 권장 alias로 제공한다.
- acceptance: 권장 alias 공개, Electron `latest.yml`과 long legacy asset 전환, 구 legacy 이름 제거, labels·release 안내, 실행형 조립 테스트와 공개 실다운로드 검증.
- risk: `HIGH — 기존 Rust 자동 업데이트와 Electron feed를 동시에 다룸`
- authorization: v0.4.2 자산 추가·feed 대치·구 legacy 자산 제거와 release 안내 변경 허용. tag rewrite와 main push 제외.
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — 공개 자산 선택과 업데이트 라우팅`
- feature_map: `updated — revision 3`
- architecture_impact: `alias, signed manifest compatibility, Electron feed sha512·blockmap`
- architecture_contract: `updated — revision 3`
