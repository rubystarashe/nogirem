# 마비노기 렘 부스터

마비노기를 조금 더 부드럽고 안정적으로 실행할 수 있도록 Windows와 그래픽 드라이버의
최적화 설정을 한곳에서 관리하는 데스크톱 앱입니다.

복잡한 설정을 사용자가 직접 찾아 적용하지 않아도 되도록 CPU 배치, 메모리 정리,
네트워크 응답 설정과 그래픽 드라이버 설정을 자동으로 확인하고 적용합니다.

- 최신 설치 파일: [GitHub Releases](https://github.com/rubystarashe/nogirem/releases)
- 제작자: 류트 서버 `[렘]`
- YouTube: [마비노기 렘](https://www.youtube.com/channel/UCb7m0UV734CHm78Mb0zEBHg)

## 만든 이유

마비노기를 부드럽게 플레이하기 위한 설정 문의가 많았지만, 사용자마다 직접 설정을
안내하거나 PC를 확인하기에는 어려움이 있었습니다. 흔히 사용하는 최적화 방법을
한 번에 적용하고 별도의 최적화 프로그램 없이도 주요 이점을 받을 수 있도록
사용자 경험 중심의 앱으로 구성했습니다.

마비노기 약관상 문제가 있거나 환경에 따라 심각한 부작용이 생길 수 있는 기능은
기본 기능에서 제외합니다. 실험적이거나 고급 사용자를 위한 기능은 설명과 사용 조건을
갖춘 고급 기능으로 분리합니다.

## 주요 기능

### 실시간 CPU 부스트

Windows CPU 토폴로지에서 물리 코어, SMT 스레드와 성능 등급을 확인합니다. 마비노기는
P-core의 절반을 우선 사용하고 일반 프로그램은 나머지 P-core와 모든 E-core를 사용하도록
배치합니다. P/E 구분이 없는 CPU도 물리 코어 단위로 절반을 나누므로 같은 코어의 SMT
스레드가 서로 다른 영역으로 갈리지 않습니다. 마비노기를 나중에 실행하거나 다른
프로그램이 새로 실행되어도 약 5초 간격으로 상태를 확인해 자동으로 반영합니다.

Windows 핵심 프로세스, 서비스, 오디오, 보안 및 안티치트처럼 변경하면 안 되는
프로세스는 대상에서 제외합니다.

### 메모리 자동 정리

마비노기 실행 중 사용 가능한 메모리와 대기 메모리를 감시합니다. 실제로 사용 가능한
메모리가 부족하고 정리할 대기 메모리가 충분할 때만 Windows 기능으로 대기 목록을
정리합니다. 다른 프로그램의 메모리를 강제로 빼앗거나 종료하지 않습니다.

### 네트워크 최적화

현재 인터넷 연결에 사용하는 네트워크 장치를 자동으로 찾아 TcpAckFrequency와
TCPNoDelay를 적용합니다. Windows TCP 자동 조정 상태는 권장값인 Normal로 맞춥니다.

선택 기능인 NIC RSS 최적화를 함께 사용하면 네트워크 처리 작업이 마비노기가 사용하는
CPU 영역과 겹치지 않도록 분리할 수 있습니다.

### NVIDIA 그래픽 최적화

마비노기 전용 NVIDIA 드라이버 프로필에 다음 설정을 적용합니다.

- 수직 동기화 끄기
- 최대 프레임 400 FPS
- 스레드 최적화 켜기
- 최고 성능 선호
- 저지연 모드 조정

다른 게임의 NVIDIA 프로필은 변경하지 않습니다.

### AMD Radeon 그래픽 최적화

AMD 공식 ADLX 기능으로 다음 설정을 적용합니다.

- 수직 동기화 끄기
- Enhanced Sync 끄기
- Radeon Chill 끄기
- Radeon Anti-Lag 켜기

ADLX의 공개 기능 제약 때문에 Radeon 설정은 마비노기 전용이 아닌 그래픽카드 전역에
적용되며 앱을 종료해도 유지됩니다. 다른 게임에도 영향을 줄 수 있으므로 앱에서
적용 전에 별도로 안내합니다. Radeon에는 최대 프레임 400 FPS를 적용하지 않습니다.

### Dioxus 전환 브랜치

이 브랜치는 Dioxus Desktop / WebView2를 사용합니다. Rust 자동 업데이트는 같은 GitHub 저장소의 서명된 `update.json`을 사용합니다.
기존 Electron은 `latest.yml`로 0.3.18 전환 버전을 받은 뒤 기존 업데이트 UI에서 Rust 전환을 진행합니다. 배포 준비는 [RELEASE_PREPARATION.md](./RELEASE_PREPARATION.md)를 확인하세요. 구조와 검증 범위는 [DIOXUS_MIGRATION.md](./DIOXUS_MIGRATION.md)를 확인하세요.

## 안전한 작동 방식

이 앱은 마비노기 실행 파일, 게임 데이터, 화면이나 통신 내용을 직접 수정하지 않습니다.
Windows와 그래픽 드라이버가 제공하는 일반 설정과 공식 기능을 사용합니다.

- CPU 배치는 Windows 작업 관리자에서도 설정할 수 있는 프로세서 선호도 기능을 사용합니다
- 메모리 정리는 Windows 대기 목록 정리 기능을 사용합니다
- 네트워크 설정은 Windows 레지스트리와 TCP 설정을 사용합니다
- 그래픽 설정은 NVIDIA 드라이버 기능과 AMD 공식 ADLX를 사용합니다
- 적용 전 상태를 기록하고 종료 시 복구 여부를 사용자가 선택할 수 있습니다

세부 원리는 [OPERATION.md](./OPERATION.md)에서 확인할 수 있습니다.

## 시스템 요구사항

- Windows 10 또는 Windows 11 x64
- 4개 이상 52개 이하의 짝수 논리 CPU
- 설정 적용을 위한 관리자 권한
- 그래픽 최적화 사용 시 지원되는 NVIDIA 또는 AMD Radeon GPU

CPU 토폴로지 조회를 지원하지 않는 환경에서는 기존 논리 CPU 절반 분할 방식으로
안전하게 대체됩니다.

## 설치 및 사용

1. [GitHub Releases](https://github.com/rubystarashe/nogirem/releases)에서 최신
   `nogirem-setup-<version>.exe`를 다운로드합니다
2. 설치 파일을 실행합니다
3. 앱 실행 시 표시되는 Windows 관리자 권한 요청을 승인합니다
4. 앱이 현재 최적화 상태를 확인할 때까지 기다립니다
5. 필요한 그래픽 및 네트워크 설정을 앱에서 적용합니다

실시간 부스트는 앱 시작 시 자동으로 실행됩니다.

- `실시간 부스트`: CPU 배치와 메모리 감시를 실행합니다
- `일시정지`: 감시를 멈추되 현재 CPU 배치는 유지합니다
- `정지(전체 복구)`: 감시를 멈추고 접근 가능한 프로세스가 전체 CPU를 사용하도록 복구합니다
- `NIC RSS 최적화 포함`: 네트워크 수신 처리를 백그라운드 CPU 영역에 배치합니다
- 고급 기능의 `Windows 시작 시 트레이 실행`: 로그인할 때 창과 시작 음악 없이 자동 실행합니다
- 고급 기능의 `터보 키`: 운영정책 안내를 확인하고 실행 파일을 선택적으로 다운로드한 뒤, 반복 대상과 1~30ms 입력 간격을 설정할 수 있습니다

터보 키와 입력·매크로 엔진은 마비노기용 P-core를 침범하지 않도록 나머지 P-core에 배치합니다.
일반 백그라운드 프로그램은 나머지 P-core와 E-core를 함께 사용합니다.

터보 키 실행 파일은 기본 설치 파일에 포함되지 않으며 사용자가 다운로드하기 전에는 실행되지 않습니다.
다운로드 파일은 현재 앱의 GitHub 정식 Release에서만 가져오고 SHA-256과 Windows x64 형식을 검증합니다.
설치 후에는 고급 기능의 `터보키 제거하기`를 눌러 실행 파일을 다시 삭제할 수 있습니다.
터보 키는 설치 후에도 기본적으로 꺼져 있고 선택된 키 없이 시작합니다.
Shift, Ctrl, Alt, Windows 키를 포함한 조합키와
잠금·시스템 키에는 적용되지 않습니다. 반복 입력 도구 사용은 게임 운영정책에 따라
이용 제한 대상이 될 수 있으므로 사용자가 해당 위험을 확인한 뒤 직접 켜야 합니다.

앱을 닫을 때 변경된 CPU 설정이 남아 있다면 다음 중 하나를 선택할 수 있습니다.

- 전체 복구 후 종료
- 현재 적용을 유지하고 종료
- 취소

현재 적용 유지를 선택해도 백그라운드 감시 프로그램은 정상적으로 종료됩니다.

## 사용 전 확인사항

- 네트워크 설정을 처음 적용하거나 복구할 때 인터넷 연결이 잠시 끊길 수 있습니다
- Process Lasso의 고정 규칙이 있으면 이 앱과 설정을 서로 덮어쓸 수 있습니다
- Radeon 최적화는 전역 설정이므로 다른 게임에 미치는 영향을 확인해야 합니다
- 접근 권한이 없는 시스템 프로세스는 변경하지 않고 건너뜁니다
- CPU 재정렬 같은 고급 기능은 안내된 실행 조건에서만 사용해야 합니다

## 앱 내 문서

- [INTRODUCE.md](./INTRODUCE.md): 제작 배경과 개발 방향
- [OPERATION.md](./OPERATION.md): 비전문가를 위한 기능별 작동 원리
- [TURBO_KEY_TERMS.md](./TURBO_KEY_TERMS.md): 터보 키 다운로드 전 운영정책 안내
- [VERSION_HISTORY.md](./VERSION_HISTORY.md): 사용자용 버전 변경 요약
- [VERSION_HISTORY_DETAIL.md](./VERSION_HISTORY_DETAIL.md): 구현 내용을 포함한 상세 변경 기록

앱의 `[류트@렘] 제작`을 누르면 위 문서와 후원 안내, 고급 기능을 앱 안에서
확인할 수 있습니다.

## 개발

개발 환경에는 Windows x64, Node.js 20 이상(빌드·테스트 도구), Rust MSVC 도구 체인, Visual Studio C++ Build Tools와
Windows SDK, WebView2 Evergreen Runtime이 필요합니다.

```powershell
npm ci
npm test
npm run test:desktop
npm run app:smoke
npm run app
```

`app:smoke`는 실제 Dioxus 창과 WebView2를 사용하지만, 최적화 서비스는 메모리상의
모의 서비스로 대체하므로 시스템 설정을 바꾸지 않습니다. 일반 `app`은 관리자 권한으로
기존 최적화 기능을 실행합니다. `app:dev`도 같은 Rust 개발 빌드를 실행합니다.

```powershell
npm run app:check
npm run app:build
npm run package:win
```

설치 패키징에는 NSIS가 필요합니다. `NSIS_MAKENSIS`에 `makensis.exe` 경로를 지정하거나
NSIS 기본 설치 경로를 사용하세요. 네이티브 헬퍼는 기존 `native:*` 명령으로 빌드할 수 있으며,
패키징은 준비된 바이너리를 그대로 사용합니다. 선택적 터보 키 헬퍼도 먼저 빌드해야 합니다.

생성 파일은 `release/dioxus-<version>-<timestamp>/`에 저장됩니다.

- `nogirem-dioxus-setup-<version>.exe`: Windows 설치기
- `app/`: Dioxus 실행 파일과 Rust 백엔드, 네이티브 헬퍼, 리소스 (Node 런타임 없음)
- `turbo-key-helper-win32-x64-v<helper-version>.exe`: 별도 배포할 선택적 헬퍼
- `build.json`: 패키지 구성 정보

설치기는 WebView2가 없으면 Microsoft 서명을 검증한 부트스트래퍼로 설치합니다.
이 경우 인터넷 연결이 필요합니다. 기존 사용자 설정과 녹화 폴더는 보존합니다.

## 배포

이 브랜치의 `release:github`은 실수로 공개하지 않도록 차단되어 있습니다. 서명된 로컬 빌드와 배포 묶음 준비는 `scripts/package-dioxus.mjs`, `scripts/prepare-release.mjs`를 사용합니다.
태그 생성, 게시, 이전 Electron 업데이트 피드 변경은 수행하지 않습니다.
Dioxus 릴리스의 버전은 `package.json`과 `desktop/Cargo.toml`에서 함께 관리하세요.
선택적 터보 키 다운로드를 제공하려면 해당 버전의 정식 Release에 별도 헬퍼 자산을
게시해야 합니다.
