# Cursor AI Handoff

Last Updated: 2026-09-30 03:16 +09:00

## Current Objective
Rust·Dioxus 0.4.2의 패스트핑 단절 처리와 업데이트·설치·제거 수명주기를 보강하고 서명 배포한다.

## Active Runs
- `20260929-release-042`: 패스트핑 기본 경로 부재, backend 건강 확인, updater/installer/uninstaller 경쟁 수정과 실서명 0.4.2 배포 완료 (`docs/ai/runs/20260929-release-042/final-report.md`)
- `20260929-cursor-minimum-042`: 25%·50% 커서 선택, 0.4.2 버전 갱신, helper 재빌드와 자동 검증 완료 (`docs/ai/runs/20260929-cursor-minimum-042/task.md`)
- `20260929-project-explain-refresh`: Rust/Dioxus 신규 개발자용 HTML 구조 보고서 생성·브라우저 검증 완료 (`docs/ai/runs/20260929-project-explain-refresh/handoff.md`)
- `20260929-portable`: v0.4.1 배포 완료, 버전 간 종단간 QA 대기 (`docs/ai/runs/20260929-portable/handoff.md`)
- `20260929-update-hotfix`: 사용자 진단 기반 0% 패닉 수정과 0.3.19 전환본 배포 완료, 실패한 0.3.18·0.4.0은 1회 수동 복구 필요 (`docs/ai/runs/20260929-update-hotfix/handoff.md`)

## Current Status
- 0.4.2 updater의 PowerShell SDDL 문자열 검사를 Win32 mandatory-label ACL 검증으로 교체하고 label authority·NO_WRITE_UP·High/System RID를 강제했다.
- Node 198개, Rust backend 57개 실행, desktop compile과 독립 재리뷰를 통과했으며 source commit은 `3cb35d86bf8967a86303b9440754445d9d338443`이다.
- GitHub v0.4.2 설치형·포터블·두 서명 manifest를 대치하고 공개 실다운로드 SHA-256과 Ed25519 연결을 검증했다.
- 진단 `55d16a69-0d32-4bfd-a118-51b03b89fab3`은 보고 시점 패스트핑 DWORD가 이미 1이었으나 인터넷 단절 중 `Get-NetRoute` 예외가 그대로 노출되는 Electron/Rust 공통 결함을 확인했다.
- 기본 IPv4 경로 부재를 구조화된 연결 없음 상태로 바꾸고, 건강 확인을 main UI 열기 성공 직후로 이동해 UI 애니메이션과 worker·PowerShell 조회에서 분리했다.
- 설치·제거 PowerShell을 숨김 실행하고, 포터블의 정상 무프로세스 `exit 1`을 제거했으며, stale updater 종료와 설치 폴더 프로세스 반복 재탐색을 구현했다.
- 이전 0.4.1 updater가 인자 없이 새 installer를 실행하는 경로도 검증된 부모 helper 하나를 보존하도록 회귀를 추가했다.
- 전체 Node 198개, Rust backend 56개 실행 중 1개 선언된 live-network ignore, Rust desktop compile을 통과했고 독립 재검토가 승인됐다.
- 실서명 설치형·포터블과 0.3.19 전환 자산을 GitHub v0.4.2 Latest Release로 공개하고 공개 manifest 서명·실다운로드 SHA-256을 검증했다.
- 커서 크기 범위를 25%~800%로 확장해 직접 선택·저장·게임 중 휠 조절이 같은 하한을 사용하며, 앱 버전을 0.4.2로 올렸다.
- 입력 helper 재빌드, 전체 Node 194개, Rust backend 56개 실행, Rust desktop 컴파일과 변경 파일 lint를 통과했다. 실제 게임에서 25% 가시성 수동 확인은 남아 있다.
- Rust/Dioxus 현재 구조를 신규 개발자 관점의 개요·ELI5·기술 구조·inline SVG 보고서로 생성했다.
- 보고서의 로컬 링크, offline 동작, SVG 접근성, 모바일·인쇄, 실제 브라우저 렌더링을 검증했다.
- `DIOXUS_MIGRATION.md`의 역사적 중간 상태 문구와 `README.md` 설치 자산명 불일치를 보고서에 기록했으며 정본은 변경하지 않았다.
- 기능 분석 후 CPU affinity·TCP autotuning·GPU 설정·설치형 rollback의 복구 한계를 보고서에 보강하고 독립 재검토 승인을 받았다.
- 앱 `0.4.1`, Dioxus 요구값 `^0.7.3`, lock resolution `0.7.10`을 구분했으며 실제 390px viewport에서 발견한 ELI5 code link 넘침을 수정했다.
- Understand Anything v2.9.0으로 Rust 앱 범위 130개 파일의 지식 그래프를 생성했다. `.ua/knowledge-graph.json`은 857 nodes, 1,296 edges, 9 layers, 10 tour steps를 포함한다.
- `understanding-report.html`은 standalone 인터랙티브 코드 그래프를 첫 화면으로 제공하며 layer, 검색, 파일 내부 함수·클래스, 관계 Inspector, guided tour를 실제 브라우저에서 검증했다.
- 공식 Understand Anything dashboard 실행은 Windows localhost bind와 Node 23/Vite 6 호환 문제로 막혔지만 동일 graph를 내장한 standalone viewer는 desktop 상호작용과 390px 반응형 검증을 통과했다.
- 최종 graph review round 1의 source/artifact target 구분·상태 기록·검색 combobox 접근성 finding을 수정했고 `REV-EXPLAIN-GRAPH-TARGET-02` 독립 재검토 승인을 받았다.
- 단일 포터블 EXE 생성과 전용 서명 업데이트 경로를 구현했다.
- 포터블 내부 앱은 관리자 전용 버전 캐시에 한 번 준비하고 다음 실행부터 재사용한다.
- 배포 EXE는 변경하지 않아 다른 위치로 복사·이동해도 독립 실행된다.
- 포터블 업데이트는 앱과 launcher 종료 후 원본 EXE를 원자 교체하고 신규 앱 준비 확인 실패 시 이전 EXE를 복원한다.
- 설치형과 포터블은 AppData 설정과 녹화 드라이브 선택을 공유한다.
- 설치형·포터블 0.4.1 매니페스트를 기존 Ed25519 키로 서명하고 GitHub Latest Release로 배포했다.
- 공개된 `update.json`과 `portable-update.json`의 서명·버전·다운로드 URL을 다시 검증했다.
- Electron 0.3.19 전환본이 기존 업데이트 UI에 Rust 진행률·오류·재시도·화면 숨기기를 표시하도록 구현했다.
- 설치·제거 종료 요청을 고유 토큰으로 반복하고 30초 뒤 현재 제거기 외 설치 폴더 내부 프로세스만 제한 종료하도록 수정했다.
- 수정된 Rust 0.4.1 설치형·포터블 실서명 산출물을 생성하고 독립 코드 리뷰 승인을 받았다.
- GitHub v0.4.1의 Rust 자산을 순차 대체하고 원격 SHA-256·Ed25519 서명·실다운로드를 검증했다.
- 실제 0.4.0 진단에서 async reqwest body가 Tokio runtime 없이 생성돼 발생한 다운로드 시작 패닉을 확정하고 순수 blocking client로 교체했다.
- Rust와 Electron 업데이트 오류 화면을 숨긴 뒤 버전 배지로 다시 열 수 있게 하고, timeout helper 종료·시도 무효화와 named mutex로 중복 전환을 차단했다.
- 승인된 설치형·포터블·전환본 7개 자산을 GitHub v0.4.1에 재배포하고 원격 해시와 공개 실다운로드를 검증했다.
- 신규 구형 Electron은 수정 helper를 받도록 전환본을 0.3.19로 올리고 라우팅과 공개 자산 해시를 검증했다. 이미 0.3.18에 진입한 사용자는 Electron 확인 경로가 Rust 전환으로 고정되어 0.3.19 수동 설치가 필요하다.
- 포터블 캐시 marker를 내부 파일 전체의 빌드 digest로 바꿔 동일 버전 교체본도 다시 준비하며, 실행 중인 기존 캐시는 삭제하지 않는다.
- 포터블 복구 작업의 High 무결성·UUID·reparse·launcher PID를 검증하고 사용자 경로 교체를 보호 staging과 열린 handle, 이동 전후 SHA-256으로 잠갔다.
- 복구·롤백된 이전 포터블 EXE의 관리자 자동 실행·시작 등록을 제거하고, 복구 기록이 없는 오래된 중단·롤백 캐시를 24시간 뒤 정리한다.
- 포터블 보강 커밋을 독립 재리뷰 승인받고 실서명 0.4.1 Rust 자산 4개를 GitHub에 대체했으며 원격 digest와 설치형·포터블 실다운로드를 검증했다.
- 늦은 리뷰와 사용자 제보로 일반 업데이트 초기 상태 누락, 상위 경로 junction 경쟁, 제거기의 자기 종료와 예약 작업 null action 실패를 확인해 추가 수정했다.
- 사용자 제보 후속 수정은 4차 독립 재리뷰에서 승인됐고 실서명 v0.4.1 자산 4개를 다시 교체했다. CDN 안정화 뒤 설치형·포터블 공개 실다운로드 검증도 통과했다.
- 고급 기능에 개발 중인 오버레이의 용도와 동작 흐름을 보여주는 인터페이스를 추가했으며 실제 다운로드·캡처 기능은 아직 연결하지 않았다.
- 앱·약관·공지의 중복 커스텀 휠 스크롤을 공통 모듈로 통합했다.
- 연속 휠 입력의 미처리 목표 거리를 화면 높이의 65%로 제한해 과도한 후속 이동을 막았다.
- 보간율을 실제 프레임 경과 시간에 맞추고 100ms 이상 프레임이 지연되면 목표 위치로 즉시 보정한다.
- 모든 드라이브 청크 정리 제한 시간을 60초로 늘리고 잔여 청크와 실행 오류를 사용자에게 유지해서 표시한다.
- Vulkan 적용 후 `latest` 이벤트가 와도 메인 화면에 이전 상태가 남던 Svelte 의존성 추적 문제를 수정했다.
- 영상 클립의 첫 청크에서 선택 이전 오디오가 재생되던 remux 시각 초기화를 수정했다.
- 송승준 리포트에 패스트핑 상태 조회 실패와 9950X3D 프레임 드랍 점검 절차를 답변했다.
- 0.3.16 전체 Node 테스트 178개와 네이티브 helper·Vite·NSIS 패키징을 통과하고 GitHub Latest Release로 배포했다.

## Architecture / Important Decisions
- 정본 기능 지도: `docs/ai/wiki/feature-map.md`
- 정본 architecture contract: `docs/ai/wiki/agent-friendly-architecture.md`
- updater가 시작한 installer는 검증된 명시 PID 또는 보호 작업 경로의 실제 부모 `updater.exe` 하나만 보존한다. 직접 설치·제거는 나머지 updater helper를 종료한다.
- `healthy.json`은 backend RPC와 main UI open 성공을 의미하며 UI 시작 애니메이션 완료를 의미하지 않는다.
- 실행 파일 옆 `portable.marker`로 포터블 모드를 감지한다.
- 기존 설치형 클라이언트 호환성을 위해 설치형 `update.json`과 포터블 `portable-update.json`을 분리한다.
- 포터블도 시스템 WebView2와 관리자 권한을 사용하며 설정은 `%APPDATA%\마비노기 렘 부스터`에서 설치형과 공유한다.
- 녹화 청크 경로는 포터블 폴더로 강제하지 않고 기존 드라이브 선택을 유지한다.
- 부드러운 스크롤은 `web/smooth-wheel-scroll.mjs`의 `createSmoothWheelScroller`를 앱·약관·공지에서 공유한다.
- 60Hz 정상 프레임에서는 기존 18% 감속감을 유지하고, 느린 프레임에서는 경과 시간만큼 더 이동한다.
- Ctrl+휠은 앱 스크롤이 가로채지 않으며 line/page 단위 휠 입력은 픽셀로 정규화한다.
- 오버레이는 터보 키처럼 앱과 분리된 선택적 다운로드 모듈로 제공할 예정이다.
- 청크 정리 완료 응답의 `bytesUsed`가 0보다 크면 성공으로 오인하지 않고 잠긴 파일 오류를 표시한다.
- Vulkan 링크의 클래스·SVG·문구는 `services.affinity.data`를 명시적 의존성으로 갖는 단일 `dxvkState` 스냅샷을 공유한다.
- `chunk-<epoch>.mp4`의 epoch가 빠른 클립 결합과 Ring 청크의 정식 시간 순서를 결정한다.
- 클립 remux 오디오는 청크 0초부터 실제 sample 시각을 추적하고 선택점 이전 PCM frame을 제거한다.

## Constraints / Rules
- 코드 주석은 한국어로 작성하고 JavaScript 줄 끝 세미콜론은 사용하지 않는다.
- 오버레이 실제 기능은 요구사항 확정 전 구현하지 않는다.
- Smart App Control 활성만으로 DXVK 설치를 막거나 DLL을 자동 삭제하지 않는다.
- 의미 있는 변경 후 이 파일을 갱신하고 설명 본문이 포함된 semantic commit을 만든다.

## Pending Tasks
1. 실패한 0.3.18에는 0.3.19, 0.4.0에는 수정된 0.4.1을 1회 수동 설치하도록 안내하고 성공 여부를 확인한다.
2. 이전 설치형·포터블 버전에서 0.4.1 업데이트 성공과 강제 실패 롤백을 종단간 검증한다.
3. 기존 0.4.1 포터블 사용자에게 수정 EXE를 1회 직접 내려받도록 안내하고 실제 성공 여부를 확인한다.
4. 0.4.x 무응답 사용자에게 보호 bootstrap 로그와 startup 로그를 받아 WebView 이전·이후 실패 단계를 판별한다.
5. 실제 앱에서 고해상도 휠과 일반 휠의 스크롤 감각을 확인한다.
6. 오버레이 모듈 배포 자산·무결성 manifest·동의 화면과 독립 다운로드/제거 IPC를 설계한다.

## Known Issues
- 이미 `신뢰할 수 없는 업데이트 작업 파일입니다`에서 막힌 0.4.1 helper는 새 payload 실행 전 실패하므로 대치 0.4.2 설치형을 한 번 수동 설치해야 한다.
- 실제 관리자 토큰의 High label 종단간 QA는 Cursor sandbox의 `icacls` 오류 1299 때문에 실행하지 못했다.
- affinity reset은 프로세스별 저장 mask 대신 현재 세션에 전체 mask를 적용하며 worker 시작 시 이전 runtime state를 삭제해 비정상 종료 전 custom affinity의 정확한 복원을 보장하지 않는다.
- 네트워크 되돌리기는 패스트핑과 RSS 원본을 다루지만 TCP receive autotuning의 이전 값은 저장·복원하지 않는다.
- NVIDIA/Radeon/VSync 최적화는 이전 설정 snapshot과 restore 경로가 없고 Radeon은 전역 설정이다.
- 설치형 업데이트 rollback은 기존 파일을 다시 복사하지만 새 설치기가 추가한 파일 제거를 보장하지 않는다.
- 이미 배포된 0.4.1 포터블은 같은 버전의 대체 자산을 자동 업데이트로 선택하지 않으므로 수정본을 한 번 직접 내려받아야 한다.
- 전체 데스크톱 스모크는 메인 viewport 640×290 검증을 통과한 뒤 기존 고급 기능 화면 진입 단계에서 `Advanced structure failed: null`로 두 번 실패했으며 시작 애니메이션 단독 스모크는 통과했다.
- 새 스크롤 보정은 자동 테스트와 빌드만 검증됐으며 실제 장치별 휠 감각 확인이 필요하다.
- 일부 DXVK 버전은 Windows Smart App Control에서 `0xC0E90002`로 차단될 수 있다.
- 0.3.16 설치본은 Authenticode 인증서 서명이 없어 Windows 검증 결과가 `NotSigned`다.
- 오버레이 버튼은 의도적으로 `개발 중` 상태다.
- 사용 중인 녹화 청크가 Windows 파일 잠금으로 삭제되지 않으면 정리 오류를 표시하며 해당 파일은 남는다.

## Key Files
- `scripts/package-dioxus.mjs`: 설치형과 포터블 산출물 생성
- `desktop/portable.nsi`: 내부 앱을 보호된 임시 런타임에 풀어 실행하는 단일 EXE launcher
- `desktop/backend/src/update_install.rs`: 설치형·포터블 업데이트 적용과 롤백
- `docs/ai/runs/20260929-portable/`: 이번 작업 요구사항·QA·리뷰·보고
- `docs/ai/wiki/portable-distribution.md`: 포터블 배포의 지속 문서
- `web/smooth-wheel-scroll.mjs`: 프레임 지연과 입력 누적을 제한하는 공통 스크롤 제어기
- `web/App.svelte`: 메인 앱과 고급 기능 오버레이 인터페이스
- `web/TermsModal.svelte`: 약관 모달 스크롤 적용
- `web/NoticeModal.svelte`: 공지 모달 스크롤 적용
- `test/smooth-wheel-scroll.test.mjs`: 정상 프레임·누적 제한·지연 보정 회귀 테스트
- `blackbox-manager.html`: 청크 정리 실행 상태와 결과 안내
- `electron/main.mjs`: 청크 정리 제어·완료 검증과 DXVK 런타임 상태 관리
- `web/App.svelte`: Vulkan 메인 화면 상태 표시
- `native/recorder-helper/main.cpp`: 녹화·청크 정렬·클립 결합
- `REPORT.json`: 진단 로그별 사용자 답변
- `VERSION_HISTORY_DETAIL.md`: 과거 구현과 배포의 상세 기록

## Recent Changes
- Electron처럼 시작 준비·음악 설정·최적화 상태 조회를 병렬 처리해 블랙박스 시작 후 메인 창이 늦게 나타나는 순차 대기를 제거했다.
- 마지막 시작 파동 발생 시점부터 2초 후 배경 파동을 시작한다.
- Windows DPI 배율은 네이티브 창 크기에만 적용하고 메인·보조 WebView 확대율은 100%로 초기화해 이중 확대와 잘림을 막는다.
- 설치형은 무인 설치 후 수동 실행일 때 앱을 자동 실행하고 updater의 `/S` 호출은 기존 건강 확인 흐름을 유지한다.
- 원본 EXE 교체 최적화는 포터블 이동성을 깨므로 철회하고 런타임 캐시만 유지했다.
- 단일 포터블 EXE의 내부 앱을 버전별로 캐시해 반복 실행의 압축 해제를 제거했다.
- 포터블 배포 형식을 ZIP에서 단일 자동 압축 해제 EXE로 교체하고 버전을 0.4.1로 올렸다.
- 포터블 관련 변경 기록을 0.4.0에서 0.4.1로 이동했다.
- 포터블 패키징과 자동 업데이트를 추가하고 설치형과 설정·녹화 드라이브 선택을 공유하도록 확정했다.
- 세 화면의 고정 프레임 18% 보간 구현을 공통 시간 기반 스크롤 제어기로 교체했다.
- 386KB까지 누적된 handoff를 현재 목표와 미완료 작업 중심으로 축약했다. 과거 상세 내역은 Git 기록과 `VERSION_HISTORY_DETAIL.md`에 유지한다.
- 배포 버전을 0.3.16으로 조정하고 오버레이 인터페이스를 `개발 중`으로 명시했다.
- 모든 드라이브 청크 정리의 10초 제한과 오류 메시지 덮어쓰기를 수정했다.
- Vulkan 경고 문구에 정상 상태의 녹색 아이콘이 함께 남을 수 있는 다중 상태 평가를 제거했다.
- 인자 없는 `dxvkLinkState()`를 `{@const}`에서 호출해 최초 상태가 고정되던 문제를 명시적 affinity 데이터 인자로 수정했다.
- 제보 클립의 첫 6.42초가 첫 청크 경계와 일치함을 확인하고 빠른 클립의 수정 시각 정렬을 시작 시각 정렬로 교체했다.
- 첫 청크의 `fileAudioTime`을 선택점으로 미리 이동시키던 로직을 제거하고 경계 sample의 앞부분도 잘라냈다. 440Hz/880Hz 합성 영상의 4초 지점 추출 결과는 880Hz 구간과 2.005초 길이로 확인했다.
- 만료된 리포트 답변 12개를 정리하고 `bcf9211a-0bab-415e-af4a-4473d2614a64` 진단 답변을 추가했다.
- 원격 REPORT 자동 정리 커밋을 병합하고 `v0.3.16` 태그와 GitHub Release를 공개했다. 네 자산의 원격 digest가 로컬 SHA-256과 일치한다.

## Next Recommended Step
0.4.x 무응답 사용자에게 `%LOCALAPPDATA%\NogiremUpdater\updates\bootstrap.log`와 `%APPDATA%\마비노기 렘 부스터\logs\startup.log`를 받아 WebView 이전·이후 실패 단계를 확인한다.
