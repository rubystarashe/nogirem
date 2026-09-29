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
