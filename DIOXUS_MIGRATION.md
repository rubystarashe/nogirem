# Dioxus Desktop 전환

## 실행 구조

- `desktop/src/ui.rs`: Rust Dioxus 메인 UI, 신호 기반 상태·설정 폼·문서·모달.
- `desktop/src/native.rs`: WebView2 창, 보조 창, 트레이, 창 이벤트와 허용된 IPC 연결.
- `desktop/src/platform.rs`: Windows 단일 인스턴스, 권한 상승, 파일 대화상자, 시스템 폴더.
- `desktop/backend/src/service.rs`: Rust 서비스와 전체 IPC 라우팅.
- `desktop/backend/src/{boost,affinity_worker,memory,network_manager,graphics,dxvk_manager,blackbox,inputs,app_services,diagnostics}.rs`: 기존 Node 백엔드의 기능 이식.
- `desktop/backend/src/rpc.rs`: stdin/stdout JSON 요청·응답. 명령용 TCP 서버를 열지 않음.
- `desktop/backend/src/bin/nogirem-cli.rs`: 기존 CPU·메모리·그래픽·네트워크·MUO CLI 명령.
- 앱은 자체 실행 파일의 `--native-service`, `--native-affinity-helper`, `--native-memory-helper` 모드를 실행한다.
- 보조 창 HTML/CSS/브라우저 JavaScript 및 기존 C++/Rust 네이티브 헬퍼는 유지한다.
- Node.js는 개발·테스트·패키징 도구에만 사용한다. 설치물에는 Node/Electron 런타임, Node addon, Node 서비스 소스가 없다.
- `src/*.mjs`, `service/*.mjs`의 구 구현은 기존 동작 대조 테스트용이다. 앱/CLI 실행 경로에서는 사용하지 않는다.

Dioxus Desktop UI의 Rust 코드는 네이티브에서 실행된다. 웹 배포용 WASM 빌드는 이 변경의 대상이 아니다.

## 창 및 설정

메인 창, 캐릭터 간소화 안내, DXVK 안내·관리, 블랙박스 관리·편집 창을 Dioxus가 소유한다.
보조 창의 preload API는 창별 허용 채널로 제한한다. 블랙박스 동영상은 전용 프로토콜과
범위 요청으로 제공하며, 한 응답을 4 MiB로 제한한다.

트레이 열기/종료, 좌클릭·더블클릭 복원, 최소화 시 보조 창 정리, 종료 시 복구/유지/취소
분기를 기존 서비스에 연결했다. 파일 대화상자는 요청한 창을 부모로 사용한다.
기존 `%APPDATA%/마비노기 렘 부스터`의 설정과 상태 기록을 계속 읽는다.
WebView2 캐시는 `%LOCALAPPDATA%/Nogirem/WebView2`에 저장한다.

디버그·릴리스의 일반 앱 실행은 Rust 호스트가 관리자 권한을 확인하고 UAC로 다시 시작한다.
서비스 계약 검사와 격리 스모크 테스트는 권한 상승 없이 실행할 수 있다. 테스트 모드에서는 권한 상승이나 최적화를 실행하지 않는다.

## 빌드와 설치

`npm run app`은 Cargo 개발 빌드를 실행한다. `npm run app:build`는 릴리스를 빌드하고,
`npm run package:win`은 준비된 네이티브 헬퍼와 함께 NSIS 설치기를 만든다.
설치기는 실행 중인 앱의 정상 종료를 최대 90초 기다리고, 실패하면 파일 교체를 중단한다.
제거 시 해당 설치에 속한 예약 작업과 패키징 때 열거한 파일만 제거한다. 사용자 데이터와
녹화 파일은 삭제하지 않는다. WebView2 런타임은 공유 런타임이므로 제거하지 않는다.

Rust 자동 업데이트는 같은 GitHub 저장소의 서명된 `update.json`을 사용한다.
Electron은 `latest.yml`의 0.3.19 전환 설치본을 거쳐 기존 업데이트 UI에서 Rust로 이전한다. 공개 전 실제 설치·복구 검증은 별도로 필요하며 배포 절차는 `RELEASE_PREPARATION.md`를 따른다.

## 검증 명령과 범위

- `npm test`: 기존 순수 로직·네이티브 소스 회귀, 실제 서비스 IPC 등록 계약, 전송 연결 끊김·시간 초과, 미디어 범위 요청.
- `npm run test:desktop`: Markdown 외부 콘텐츠 처리 등 Rust 단위 테스트.
- `npm run app:smoke`: 실제 WebView2에서 상태→DOM 갱신, 종료 취소, 5종 보조 창과 내장 편집기 초기화, 리소스 로드, 트레이 복원, 터보 키·커서·CPU 설정 폼 왕복.
- `npm run package:win`: 릴리스 링크와 설치기 생성. 설치 실행은 별도 검증 대상.

Svelte 구문·Electron 배포 설정에 종속된 소스 문자열 검사는 제거/이관했다.
따라서 기존 테스트 개수만으로 UI 동등성을 판단하면 안 된다. WebView 스모크 테스트는
실제 창을 사용하지만 OS 작업은 모의 서비스다. 다음은 실제 환경에서 별도로 확인해야 한다.

- 마비노기 실행 중 CPU/NIC/메모리 최적화 적용·복구, NVIDIA/AMD 드라이버별 동작.
- 실녹화, 오디오·비디오 재생 및 클립 추출, 저장 중 종료/최소화 제한.
- 다중 모니터·서로 다른 DPI, Windows 10, WebView2 미설치 상태에서 설치.
- 로그인 예약 작업, 관리자 권한 취소, 설치 업그레이드·제거 및 WebView 프로세스 비정상 종료.
- 시작 애니메이션·색 전환·보조 창 포커스의 세부 시각 동등성.

현재 자동 검증은 위 항목 전체의 완전한 동작 동등성을 입증하지 않는다.

## Rust 백엔드 검증 (2026-09-29)

Rust 단위 테스트 외에 원본 계산 결과 대조 데이터 259개를 검사한다. 실제 Rust 서비스 통합 테스트는
독립 데이터 폴더와 존재하지 않는 게임 경로를 사용하여 설정 저장, IPC 접근 제한, 영상 Range,
CPU/메모리 워커 시작·종료를 확인한다. 청크 삭제와 MUO 설치, 진단 ZIP은 임시 파일로 검증한다.
실제 게임에서의 녹화·입력 주입·드라이버 설정 적용은 이 자동 검증에 포함되지 않는다.

## UI 회귀 수정 (2026-09-28)

원본 SVG, 고급 기능 인라인 설정, 키보드 배치·물리 키 선택, 모달·탭·시작 전환을 복원했다.
시작 데이터 준비 전에 부분 상태 알림으로 애니메이션이 시작되던 문제, MP3 Range 응답 누락,
파문의 서로 다른 시간 기준, 보조 창의 초기 숨김 상태, 블랙박스 부분 상태 덮어쓰기를 수정했다.
`app:smoke`는 이제 음소거하지 않은 시작 경로도 실행하여 MP3 지정 구간 재생·볼륨,
캔버스 파문 변화, 중앙 배치와 숨김 보조 창을 확인한다. 실제 서비스 연결에서도
기존 설정 로드, 시작 음악 설정 저장·복원, 고급 기능 화면을 확인했다.
게임이 실행 중이어야 하는 CPU 재정렬·실녹화의 완전한 동등성은 이 검증으로 단정하지 않는다.

## 원본 UI 대조 수정 (2026-09-28, 후속)

원본 Svelte 코드를 Git HEAD에서 별도 참조 폴더로 추출해 같은 640×290 크기로 실행했다.
전역 `web/styles.css`는 변경하지 않고 원본 태그 구조·클래스·SVG·문서 블록을 Dioxus에 맞췄다.

- 최적화 상세: 원본 large 모달, 두 카드의 헤더·버튼·장치 정보·복원 링크 복구. 가로/세로 넘침 0.
- 블랙박스: 원본 활성 배경·카메라 아이콘·누적 시간·상태 전환 구조 복구.
- 소개 탭: 원본 이름·순서·채널 카드, 작동 원리·후원·버전 기록 구조 복구.
- 고급 기능: 가로 스크롤 제거, 원본 보조 버튼 스타일, CPU 코어 한 줄 배치 및 비지원 상태 복구.
- 약관/공지: 별도 컴포넌트의 원본 구조와 CSS 복구. 모달 제목 CSS가 본문까지 침범하지 않도록 범위 제한.

참조 UI와 일치한 내용 영역 치수(폭 × 높이 / scrollHeight):
소개 429×212 / 569, 작동 원리 429×212 / 2040, 후원 432×212 / 212, 고급 기능 429×212 / 974.
이 수치와 캡처 비교는 해당 모의 상태에서의 검증이며 모든 시스템·데이터·전환 프레임의 픽셀 동일성을 뜻하지 않는다.

Rust 5개, Node 178개 테스트 통과. WebView2 회귀 검증에서 모달 범위·탭 전환 중 가로 스크롤·자산,
설정 이벤트 왕복·5종 보조 창·트레이·음소거하지 않은 시작 음악·파문·중앙 배치·숨김 예열을 확인했다.

## 창 배치 및 블랙박스 후속 수정

보조 창은 메인 창 모니터의 물리 작업 영역 중앙에 표시한다. 안내 창 드래그는 Windows 기본
창 이동으로 처리하고, 예전 수동 좌표 이동 알림은 무시한다. 실제 중앙 오차는 (0, -1) 픽셀.
Tao의 undecorated shadow 기본 설정이 만드는 상단 1픽셀 선을 끄고 네이티브 테두리를 제거했다.

기존의 “최근 6시간 중 7초만 녹화됐다”는 진단은 잘못이었다. index 조회가 마지막 파일과
영상 형식이 같은 파일만 남기는 필터를 적용하여 기존 파일들을 누락했다. 해당 필터와
6시간 조회 제한을 제거했다. 초기 조회는 원본처럼 최신 녹화 완료 시점 기준 15분이며, 전체 조회는 ++ 팝업에서 선택한다. 전체 조회 검증에서는 실제 1,791개 파일,
총 17,992.165초(4시간 59분 52초)를 확인했다. 처음부터 마지막까지 경과 시간은
123,689초이며, 녹화하지 않은 공백을 포함하므로 실제 녹화 시간 합계와 다르다.

원본 녹화 실행 파일은 변경하지 않고, 수정된 조회·추출 도구를 tools-bin에 별도 빌드한다.
명령: node scripts/build-recorder-helper.mjs --tools-only. 패키징 시 이 도구도 포함한다.
오래된 파일과 최신 파일 각각에서 2초 추출을 확인했다. 실제 WebView2에서 3440×1440
영상 디코딩(readyState 4)을 확인했다. 게임 실행 중 신규 녹화는 아직 검증하지 않았다.
서로 다른 영상 형식을 한 클립으로 합치는 변환 기능을 추가한 것은 아니다.

전체 조회 버튼은 기존 하단 배치를 유지하도록 시간 직접 설정(++) 팝업 안에 배치했다.
블랙박스 관리자 HTML과 기존 편집기 스타일을 유지하고, 공백 안내 및 전체 조회 조작만 추가했다.
## 시작 애니메이션 원본 순서 복구

GameWave의 시작 전용 로고·앱 이름 오버레이가 이관 과정에서 누락된 것을 복구했다.
브랜드→버전의 300ms CSS 애니메이션 완료 이벤트 후 1500ms를 유지하고 상태 문구로
전환한다. 이후 2030ms에 좌측 항목, 3000ms에 제작자 버튼을 표시한다.
간소화/Vulkan 링크의 entered 상시 적용과 visibility 우회 규칙을 제거하여 원본의
100ms 지연 및 450ms opacity/translate 전환을 사용한다.
실제 WebView 회귀 측정: 좌측 2036ms, 제작자 3006ms. 시작 로고/문구 존재,
링크의 중간 opacity, 시작 오디오 재생, 파문, 중앙 배치 및 숨김 보조 창 검증 통과.

## 보조 창 페이드인 및 종료 잔상 수정

Tao의 표시/항상 위 상태 변경이 GWL_EXSTYLE을 다시 작성하면서 WS_EX_LAYERED를 지웠다.
이 때문에 다음 투명도 갱신 전까지 창이 불투명하게 표시될 수 있었다. 앱 창의
WM_STYLECHANGING에서 해당 플래그를 유지하고, 매 프레임 불필요한 스타일 재설정을 제거했다.
보조 창에서 alpha 0/102를 설정한 뒤 hide/show/topmost/focus를 변경해도 alpha가 유지되는지
실제 HWND의 GetLayeredWindowAttributes로 확인한다. 전체 WebView 스모크 검증 통과.
종료는 네이티브 창을 숨기고 DWM 합성을 기다린 뒤 웹뷰를 해제하도록 변경했다.

## 실제 배포본 문구·마우스 추적 대조

설치된 Electron app.asar의 활성 main-RoJpSYqf.js까지 확인했다. 중간 시작 문구는
숫자 버전이 아니라 '공개 사용자 버전'이다. 앱 이름 → 공개 사용자 버전 → 부스트 상태
순서를 복구했다. 버전 표시 전환을 없앴던 직전 수정은 잘못이므로 철회했다.
테두리는 사용자가 제공한 기존 화면의 실측 RGB(200,200,200), #C8C8C8로 맞췄다.

배포본의 배경 이동 배율 /20, /10을 유지하며, 프레임당 0.05 보간을 60Hz 기준의
경과 시간 보간으로 변경했다. 캔버스 크기에 맞게 포인터 좌표를 변환하고 캡처 단계에서
수신한다. 실제 WebView 고정 좌표 검증에서 1.5초 후 우하단 이동량 x>=15.5,y>=14를 확인했다.
기존 배포 앱과의 체감 응답 속도 비교 전체를 자동 검증했다는 뜻은 아니다.

## 시작 테두리 및 로고 전환 프레임 수정

시작 캔버스 위의 ::after 레이어에 테두리를 그려 처음부터 보이도록 했다.
로고 마스크 표시 상태를 Rust 신호로 왕복시키지 않고, 캔버스를 그리는 동일한
JavaScript 프레임에서 html 클래스와 DOM 로고 표시를 전환한다. 마스크 종료 시
0.7에서 대기 상태 투명도까지 600ms로 이어지도록 했다.
실제 WebView 프레임 검사에서 마스크/DOM 표시 불일치 0, 중간 투명도 표시,
시작 테두리 색상, 공개 사용자 버전 문구 및 마우스 이동량 검증 통과.

## 2026-09-29: tray menu routing

- Reproduced the Open failure by sending WM_COMMAND for the real retained tray menu through muda, rather than notifying the service directly.
- Dioxus Desktop 0.7.10 and tray-icon share muda 0.17.2. MenuEvent uses OnceCell; Dioxus registers its menubar receiver first, so tray selections arrive as MudaMenuEvent. Handle both menu routes, filtering nogirem item IDs.
- Disable Dioxus automatic show-all-windows on tray click. The existing service owns restoration, taskbar visibility, focus, and auxiliary-window cleanup.
- Regression covers native menu Open restoring the hidden main window, Exit displaying the confirmation, and cancel leaving the UI usable. The same test failed before the handler fix and passed afterward. Full WebView2 smoke/startup suite and native service IPC contract passed.
- Production service exit policy remains unchanged: applied-boost confirmation, hidden-window native dialog, keep/reset/cancel handling, and orderly helper shutdown. Smoke uses isolated service fixtures; it does not stop real recordings or reset optimization settings to test exit choices.

## Rust backend migration in progress (2026-09-29)

The earlier installer is NOT a complete Rust backend migration: it bundles Node.js.
Do not describe this branch or rebuild as Node-free until the remaining controller is replaced.

Implemented and connected to production source:
- Native CPU topology query through `backend.cpu-allocation`; the transitional JS adapter restores BigInt masks, preserving bit operations beyond 32 bits.
- Rust affinity worker (`--native-affinity-helper`) with Win32 process enumeration/affinity, original-mask records, game exit restoration, keep/reset control, core reconfiguration, CPU reorder, NIC integration, renderer detection and status updates. Node affinity worker removed.
- Rust memory worker (`--native-memory-helper`), native memory status and standby purge APIs, pressure/rearm/cooldown policy, parent monitoring, duplicate-worker locks and graceful stop. Node memory worker and FFI status reader removed from service.
- Rust network/TCP/NIC operations (`--native-operation`), preserving the original Windows PowerShell management scripts as compile-time assets. Controller calls the native binary for these operations. No network configuration was changed for testing.
- Rust atomic JSON writer, process identity checks and helper locking.

Implemented but not yet connected to all production callers:
- Blackbox setting/shortcut normalization, bitrate/quality selection and turbo key normalization.

Still Node-dependent and REQUIRED before removing runtime/node.exe:
- Service controller, window/tray orchestration messages, preferences and startup task management.
- NVIDIA/Radeon graphics orchestration and DXVK install/update management.
- Blackbox helper orchestration, storage enumeration/cleanup, editor sessions, extraction/media protocol and clips.
- Input guard/turbo helper orchestration and installation.
- Notices, creator profile, report replies, diagnostic archives and remaining file operations.
- `desktop/src/bridge.rs` still starts the Node controller. Packaging still includes runtime/node.exe and node_modules.

Validation:
- 196 fixed cases generated from original JS: CPU masks/topology, blackbox settings/quality/bitrate and turbo key normalization.
- Rust backend unit tests include read-only native process/memory queries, pressure hysteresis, malformed topology, atomic writes, network write guards, validation-before-restart, PowerShell UTF-8/timeout/output cap.
- Isolated native worker integration: fake nonexistent game, NIC disabled, no initial memory purge; checked status, duplicate protection, graceful keep/stop and lock cleanup. Read-only TCP auto-tuning query through the native executable passed.
- Existing 178 Node regression tests passed. WebView2 smoke suite passed during the migration; its isolated service fixture does not prove real optimization parity.
- Live game optimization, affinity reset/reorder and network mutation were not exercised against the user's running programs.
- No new release installer has been built for this incomplete backend migration.

## Escape key regression check (2026-09-29)

- Fixed auxiliary Escape routing to let renderer modal/shortcut handlers consume the key before closing the native window. Included same-origin editor iframe events and ignored auto-repeat.
- Main modal close now ignores repeat keys and stale delayed close requests, preserving the underlying settings panel.
- Production and fixture share `service/window-escape.mjs`; `web/window-escape.js` is included in desktop build assets.
- `--smoke-test --smoke-escape` checks 26 cases: five auxiliary windows, focused embedded editor, delete/export modals, shortcut capture, ten main dialogs, terms/notice/report dialogs, busy guard and repeat keys.
- `--smoke-native-keys` uses Windows WM_KEYDOWN/WM_KEYUP directed only to the tested WebView's focus HWND; repeat cases use DOM KeyboardEvent. All 26 passed. Global SendInput was not used after foreground activation was denied.
- Test fixtures open modal state without deleting recordings, exporting clips, changing optimizer settings or downloading helpers. This verifies key routing and dismiss behavior, not those destructive/long-running operations.
- Existing 179 Node tests passed. Changes built in debug; the previously supplied installer is not updated by this check.
