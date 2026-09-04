# nogirem

마비노기가 실행되는 동안에만 Windows CPU Affinity를 임시 분리하는 Node.js 도구입니다.

- `C:\Nexon\Mabinogi\Client.exe`: 논리 CPU의 뒤쪽 절반
- 현재 로그인 세션의 일반 사용자 앱: 앞쪽 절반
- Windows·서비스·오디오·보안/안티치트 프로세스: 변경하지 않음
- 마비노기나 도구 종료 시: 변경 전 Affinity로 복원

## 요구사항

- Windows x64, Node.js 20 이상
- 4~52개의 짝수 논리 CPU

현재 버전은 논리 CPU 번호의 앞·뒤 절반을 사용합니다. 동종 SMT CPU를 대상으로 하며 Intel P/E 코어 같은 하이브리드 CPU에서는 물리 코어 구성이 기대와 다를 수 있습니다.

## 실행

Mabinogi가 관리자 또는 안티치트 보호 상태로 실행되는 경우 일반 터미널에서는
affinity 변경이 거부될 수 있습니다. 이때는 `nogirem-admin.cmd`를 실행하고 UAC를
승인하세요. 이미 열려 있는 일반 권한 `npm start` 창은 먼저 Ctrl+C로 종료합니다.

간소화 로그는 관리자 터미널에서 `npm run start:quiet`로 시작하거나
`nogirem-quiet-admin.cmd`를 실행합니다. 게임 감지 시 `마비노기 프레임 부스트 On`,
게임 또는 nogirem 종료 시 `마비노기 프레임 부스트 Off`만 표시합니다.

마지막 물리 코어 하나만 게임에 격리하는 실험 모드는 `npm run start:last-core`로
실행합니다. 간소화 로그는 `npm run start:last-core:quiet`입니다. 관리자 실행 파일은
각각 `nogirem-last-core-admin.cmd`, `nogirem-last-core-quiet-admin.cmd`입니다. 16논리 CPU
환경에서는 게임이 CPU 14~15(`0xC000`), 백그라운드가 CPU 0~13(`0x3FFF`)을 사용합니다.

패시브 모드는 마비노기 affinity를 변경하지 않고, 게임 실행 중 다른 대상 프로그램만
뒤쪽 절반의 CPU로 옮깁니다. 16논리 CPU에서는 백그라운드가 CPU 8~15(`0xFF00`)를
사용합니다. `npm run start:passive` 또는 `nogirem-passive-admin.cmd`로 실행하며,
간소화 버전은 `npm run start:passive:quiet` 또는 `nogirem-passive-quiet-admin.cmd`입니다.

현재 사용자 세션에서 접근 가능한 프로세스의 affinity를 전체 논리 CPU로 되돌리려면
nogirem을 먼저 종료한 뒤 `npm run reset` 또는 `nogirem-reset-admin.cmd`를 실행합니다.
Process Lasso의 Always 규칙은 삭제하지 않으므로 활성 규칙이 있으면 다시 적용될 수 있습니다.

선택형 메모리 정리는 `npm run start:memory` 또는 `nogirem-memory-admin.cmd`로 실행합니다.
마비노기 실행 중에만 1초마다 상태를 확인합니다. 사용 가능 RAM 발동선은 총 RAM의
10%(최소 768MB, 최대 2GB), Standby 발동선은 총 RAM의 5%(최소 256MB, 최대 1GB)로
자동 계산하며 두 조건이 동시에 맞을 때만 정리합니다. 정리 후 최소 10분이 지나고 사용
가능 RAM이 총 RAM의 15%(최소 1.5GB, 최대 4GB)까지 회복되어야 다시 실행됩니다.
기본 실행에는 포함되지 않습니다. 현재 상태는 `npm run memory:status`로 확인합니다.

기본 실행은 연결된 IPv4 기본 경로 중 합산 메트릭이 가장 낮은 인터페이스를 주 사용
인터페이스로 판별합니다. 해당 인터페이스에 `TcpAckFrequency=1`, `TCPNoDelay=1`이
없으면 관리자 권한 실행에서 자동 적용하고 다시 읽어 검증한 다음 인터페이스를 한 번
재시작합니다. 조회만 하려면 `npm run fast-ping:status`를 사용합니다. 값이 이미 있어도
인터페이스를 강제로 재시작하려면 `nogirem-fast-ping-admin.cmd` 또는 관리자 터미널에서
`npm run fast-ping:restart`를 실행합니다. 재시작하는 동안 네트워크 연결이 잠시 끊깁니다.

Windows 전역 TCP 수신 창 자동 조정 수준은 `Normal`만 최적화 상태로 판정합니다.
`Disabled`, `Restricted`, `HighlyRestricted`, `Experimental`이면 기본 관리자 실행에서
`Normal`로 복구하고 재조회합니다. 조회는 `npm run tcp:status`, 별도 복구는 관리자
터미널에서 `npm run tcp:apply`를 사용합니다. 그룹 정책이 다른 값을 강제하면 복구
실패로 보고 정책 값을 함께 표시합니다.

마비노기 경로는 실행 중인 `Client.exe`에서 자동으로 읽습니다. 실행 파일명이
`Client.exe`이고 전체 경로에 `Mabinogi` 폴더가 포함되면 설치 드라이브와 무관하게
감지합니다. `config.json`의 `gameExecutable`은 기존 설치 경로를 위한 보조 판별값입니다.

로그의 `[already:background]`와 `[already:game]`은 Process Lasso 등에서 같은
affinity가 이미 적용되어 있어 nogirem이 변경할 필요가 없었다는 뜻입니다.
`[skip-permanent]`는 해당 PID와 시작 시간의 프로세스 인스턴스에는 다시 시도하지
않는다는 뜻입니다. 같은 프로그램이 새 PID로 재실행되면 새 대상으로 한 번 처리합니다.

```powershell
npm install
npm run self-test
npm run dry-run
npm start
```

## 데스크톱 최적화 앱

Electron과 Svelte로 구현한 화면에서 프레임 부스트, NVIDIA 프로필, 네트워크
최적화 상태를 한 번에 확인하고 필요한 항목만 적용할 수 있습니다. 프레임 부스트
카드는 Affinity 코어 분리와 메모리 최적화를 함께 표시하고 제어합니다.

```powershell
npm run app:build
npm run app
```

개발 중에는 `npm run app:dev`를 사용합니다. 승격된 개발 앱이 Vite를 계속 사용할
수 있도록 터미널의 개발 서버는 `Ctrl+C`로 종료할 때까지 유지됩니다. 앱은 시작할 때
UAC를 한 번 요청하고 관리자 권한으로 재실행됩니다. 이후 Affinity와 네트워크
최적화 버튼에서는 UAC를 다시 요청하지 않습니다. 패스트핑 레지스트리를 변경한
경우에만 네트워크 인터페이스를 재시작합니다. NVIDIA 최적화 버튼은 기존 NVIDIA
모듈의 상태 확인·적용 함수를 그대로 사용합니다.

프레임 부스트는 앱 시작 시 자동 실행됩니다. `실시간 부스트`는 Affinity와 메모리
helper를 동시에 실행하고, 같은 버튼의 `일시정지`는 두 감시를 함께 끝내면서 이미
적용된 CPU 배치를 유지합니다. `정지(전체복구)`는 메모리 감시를 중지하고 접근 가능한
현재 사용자 세션 프로세스를 모든 논리 CPU 사용 상태로 되돌립니다. `NIC RSS 최적화
포함`을 선택하고 실시간 부스트를 실행하면 RSS 수신 처리도 백그라운드 CPU 범위에
배치합니다. 이 앱이 NIC RSS를 실제로 변경한 경우에만 전체 복구에서 저장된 원본을
복원하며, NIC가 이미 원본과 같으면 네트워크 인터페이스를 재시작하지 않습니다.

창 닫기 버튼을 누르면 웹 모달에서 `전체 복구 후 종료`, `현재 적용 유지하고 종료`,
`취소` 중 하나를 선택합니다. 적용 유지를 선택해도 helper 감시는 정상 중지되며,
현재 프로세스의 CPU 배치만 그대로 남습니다.

메모리 helper는 마비노기 실행 여부와 사용 가능 RAM·Standby RAM을 감시하고 기존
임계치와 재발동 대기 정책에 따라 필요한 경우에만 Standby 목록을 정리합니다.

## NVIDIA 3D 설정 확인

```powershell
npm run nvidia:status
npm run nvidia:apply
```

NVIDIA GPU와 드라이버를 감지하고 `Client.exe` 프로필의 수직 동기화, 최대 프레임
속도, 스레드 최적화, 전원 관리 모드, 저지연 모드를 읽기 전용으로 확인합니다.
프로그램 프로필에 값이 없으면 전역 또는 드라이버 기본값을 표시합니다.
수직 동기화가 응용 프로그램 제어 상태이면 마비노기의
`HKCU\Software\Nexon\Mabinogi\VerticalSync` 값까지 읽어 최종 구성을 판정합니다.

저지연 모드는 NVIDIA 공개 NVAPI 설정에 포함되지 않으므로 NVIDIA Profile Inspector가
사용하는 제어판 상태와 드라이버 활성화 값을 함께 확인합니다. 이 명령은 설정을 변경하지
않습니다.

`nvidia:apply`는 마비노기 프로그램 프로필에 수직 동기화 끄기, 최대 프레임 속도
400 FPS, 스레드 최적화 켜기, 최고 성능 선호, 저지연 모드 울트라를 적용합니다.
저지연 모드의 실제 드라이버 경로에 필요한 최대 사전 렌더링 프레임도 1로 설정하고,
마비노기 자체 `VerticalSync` 값도 끕니다. 저장 후 모든 목표값을 다시 읽어 검증하며
실행 중인 게임에는 다음 클라이언트 실행부터 확실히 반영됩니다.

## NIC RSS CPU 분리

```powershell
npm run nic-rss:status
npm run nic-rss:apply
npm run nic-rss:restore
```

주 IPv4 네트워크 어댑터의 RSS 상태와 처리 CPU 범위를 affinity 기능과 별도로
진단합니다. 16개 논리 CPU에서는 마비노기 영역 8~15와 겹치지 않도록 RSS를
CPU 0~7, 프로세서 그룹 0의 `ClosestStatic` 프로필로 설정합니다.

적용 전 원본 RSS 설정은 `%LOCALAPPDATA%\nogirem\nic-rss-state.json`에 저장하며
`nic-rss:restore`로 되돌릴 수 있습니다. 적용과 복원에는 관리자 권한이 필요하고
네트워크 인터페이스가 잠시 재시작됩니다. 이 기능은 RSS 수신 처리를 조정하며
장치 IRQ affinity 자체를 강제로 변경하지 않습니다.

`dry-run`은 대상을 출력만 하고 `start`는 실제 적용합니다. 게임이 없을 때는 변경하지 않습니다. 게임이 종료되면 원복하고 다음 실행을 기다립니다. 도구 종료는 `Ctrl+C`를 사용합니다.

같은 프로세스에 Process Lasso 규칙이 남아 있으면 양쪽 프로그램이 서로 설정을 덮어쓸 수 있습니다. 실제 사용 전 `client.exe`와 백그라운드 앱의 기존 Lasso Affinity 규칙을 비활성화하세요.

설정은 `config.json`에서 바꿀 수 있습니다. 접근할 수 없는 프로세스는 건너뜁니다.

적용 중에는 `runtime-state.json`에 PID·시작 시각·원래 마스크를 기록합니다. 비정상 종료 후 다음 실행에서도 PID와 시작 시각이 모두 일치할 때만 원복합니다.
