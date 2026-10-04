# Cursor AI Handoff

Last Updated: 2026-10-05 07:23 +09:00

## Current Objective
0.4.4 채널별 핑 신규 기능 공지를 원격 이미지와 함께 테스트 앱에 열어 두었다. 공지 본문은 사용자 화면 확인 전 원격 공개하지 않는다.

## Active Runs
- `20261005-notice-044`: package에서 제외되는 공지 이미지만 origin/main `05e6d59`에 게시했고, `NOTICE.md`와 원본 비율 `cover` 스타일을 production renderer로 preview했다. fixture의 신규 channel ping getter 누락을 수정했으며 preview PASS·앱 열린 상태다. `NOTICE.md` 공개는 사용자 화면 확인 뒤다. (`docs/ai/runs/20261005-notice-044/task.md`)
- `20261005-release-044`: source commit `60f9b58cea09c4d1668af56c7cfda446211b4a51`에서 signed 설치형·포터블과 update manifest를 생성·검증했다. 후보는 `release/dioxus-0.4.4-2026-10-04T21-52-05-572Z`; 설치형 SHA-256 `1584e3fc...`, 포터블 `6425e5d0...`. 실제 게임·설치 수동 QA는 NOT_RUN이며 배포·push·tag는 금지다. (`docs/ai/runs/20261005-release-044/task.md`)
- `20261005-channel-ping-overlay`: startup GitHub `channel.csv` 검증·cache fallback, 1분 측정·최근 5회 평균, 실패 중 1시간 재검사, 마비노기 전경 Windows 키 표시와 모든 후속 down 숨김, click-through topmost 보조 창, 고급 기능 toggle을 구현했다. 자동 검사와 독립 review 승인, source commit `000481dbdc438729d13b54829d4653ab51824c78` 완료 (`docs/ai/runs/20261005-channel-ping-overlay/task.md`)
- `20261002-dxvk-043-replacement`: 잘못된 게임 경로·reparse를 차단하고 handle 기반 DXVK 교체·backup 복구·보안/권한 오류 안내를 구현했다. 독립 review, 자동·서명 package·공개 재다운로드 검증, GitHub v0.4.3 자산 10개 대치와 origin/main push 완료 (`docs/ai/runs/20261002-dxvk-043-replacement/task.md`)
- `20261002-release-043`: 게임 종료·helper 종료 뒤 stale 부스트 상태와 중단 조작 차단을 수정하고, 테스트 서버 `Mabinogi_Test\Client.exe`를 터보키 0.1.8에서 지원한다. 자동·package·공개 재다운로드 검증과 독립 review 승인 후 GitHub v0.4.3 배포 완료 (`docs/ai/runs/20261002-release-043/task.md`)
- `20261001-repackage-042`: 진단 로깅 포함 Rust/Dioxus 0.4.2 설치형·포터블 서명 패키징, 기존 v0.4.2 자산 대치와 공개 검증 완료 (`docs/ai/runs/20261001-repackage-042/task.md`)
- `20261001-freeze-diagnostics`: 게임 종료·재실행 하드 프리즈의 다음 진단에서 affinity 전환과 WHEA·GPU·LiveKernelEvent 증거를 수집하도록 로깅 구현·검증 완료, implementation commit `1eb7fe44e4100d6640d232ba03ce8dd9926e6fc9` (`docs/ai/runs/20261001-freeze-diagnostics/task.md`)
- `20260929-release-042`: 패스트핑 기본 경로 부재, backend 건강 확인, updater/installer/uninstaller 경쟁 수정과 실서명 0.4.2 배포 완료 (`docs/ai/runs/20260929-release-042/final-report.md`)
- `20260929-cursor-minimum-042`: 25%·50% 커서 선택, 0.4.2 버전 갱신, helper 재빌드와 자동 검증 완료 (`docs/ai/runs/20260929-cursor-minimum-042/task.md`)
- `20260929-project-explain-refresh`: Rust/Dioxus 신규 개발자용 HTML 구조 보고서 생성·브라우저 검증 완료 (`docs/ai/runs/20260929-project-explain-refresh/handoff.md`)
- `20260929-portable`: v0.4.1 배포 완료, 버전 간 종단간 QA 대기 (`docs/ai/runs/20260929-portable/handoff.md`)
- `20260929-update-hotfix`: 사용자 진단 기반 0% 패닉 수정과 0.3.19 전환본 배포 완료, 실패한 0.3.18·0.4.0은 1회 수동 복구 필요 (`docs/ai/runs/20260929-update-hotfix/handoff.md`)

## Current Status
- 채널 핑은 고정 GitHub raw URL을 startup에 확인하고 유효한 CSV만 `%APPDATA%\마비노기 렘 부스터\channel-ping\channel.csv`로 원자 교체한다. 원격 실패 시 cache→package 순으로 복구한다.
- 기능 활성 중 endpoint가 있는 채널을 1분마다 병렬 TCP 연결하고 최근 성공 5회 평균을 메모리에 유지한다. 빈 11채널은 `정보 없음`이며 실패 갱신을 유발하지 않는다.
- 실제 endpoint 실패가 있으면 startup 확인 뒤 최대 1시간에 한 번 원격 CSV를 다시 확인한다. 기능 비활성 중에는 TCP 측정과 저수준 input hook을 모두 중단한다.
- 기능 활성 중에만 `WH_KEYBOARD_LL`·`WH_MOUSE_LL`을 전용 message thread에 설치한다. callback이 Windows 키 down 순간의 전경 창을 캡처하므로 지속적인 전역 키 상태 조회나 Windows shell 포커스 경쟁에 의존하지 않는다.
- 검증된 마비노기 전경에서 좌·우 Windows 키 down 시 client rect·DPI로 중앙 위치를 계산해 584×400 반투명 click-through·비활성·topmost 창을 표시한다.
- 표시 trigger 당시 눌린 입력은 release까지 무시하며 그 외 keyboard·mouse down edge에 창을 숨긴다. 고급 기능에서 상태를 저장해 켜고 끈다.
- 수동 활성화에서 발견된 `Unknown auxiliary document`는 Dioxus native 허용 목록의 `channel-ping-overlay.html` 누락이 원인이며 수정했다. 정지 화면의 `터보키 제거하기` 버튼은 피드백에 따라 최소 너비 72px·좌우 5px 여백으로 축소했다.
- 표시 뒤 입력이 닫히지 않던 상태 경쟁을 피하도록 input monitor loop가 성공한 show/hide 상태와 native window ID를 추적한다. 실제 QA에서 여전히 남은 문제는 검증된 `(window ID, HWND)`를 hook에 등록해 다음 down callback에서 즉시 숨기고, registry 제거 전 등록을 해제하는 방식으로 보강했다.
- 오버레이 외곽 여백·잘리는 그림자를 제거하고 채널을 `1~15`, `16~29`, `30~38` 세로 3열로 재배치했다. 각 열 폭을 170px로 줄이되 행 내부 채널 왼쪽·핑 오른쪽 정렬은 유지하고, 전체 창 폭도 콘텐츠에 맞춰 584px로 축소했다. endpoint가 없는 채널은 숨기며 우상단 로고, 실시간 `n초 전` 표시, 5/10/15ms 색상 경계와 최저 핑 단일 강조를 적용했다.
- 후속 자동 검사는 channel ping 관련 11/11, overlay-interface 3/3, desktop locked check, JavaScript syntax, 변경 파일 lint 0을 통과했고 HWND 수명 경합 수정까지 독립 재리뷰 `APPROVED`를 받았다. 최신 빌드의 물리 입력 즉시 숨김과 최종 시각 확인은 남아 있다.
- channel ping 8/8을 기본 병렬 설정으로 10회 반복했고 backend 79 PASS·2 declared ignore와 integration 2/2, Node 203/203, desktop locked check가 통과했다. 독립 review는 모든 must-fix 해결 후 `APPROVED`다. 실제 게임·DPI·물리 입력과 package 실행은 NOT_RUN이며 push·배포는 하지 않았다.
- DXVK install은 실제 `Client.exe`, 허용 폴더·launcher, canonical parent와 파일·상위 reparse를 다운로드 전에 확인하고, file replace 직전에 경로 변경·게임 실행을 다시 차단한다.
- 저장소와 게임 폴더 DLL은 디렉터리·staged·기존 파일 handle identity를 유지·재확인하고 UUID 임시 파일과 기존 파일 backup을 flush·hash 검증한 뒤 staged handle의 `SetFileInformationByHandle`로 교체한다. 외부 deployment I/O 오류·불일치까지 backup을 유지하며 복구 실패 시 수동 복구용으로 보존한다.
- 임시·최종 파일 `NotFound`는 Windows 보안·백신 격리 가능성을, access denied·sharing violation은 보안 차단·권한·파일 사용을 구분해 안내한다.
- round 4 backend 71 PASS·2 declared ignore와 integration 2/2, DXVK 14/14, Node 203/203, desktop locked check가 통과했다. 독립 review round 4가 exact source target을 `APPROVED`했으며 unresolved mandatory finding은 0이다.
- source commit `567ab935aad7d69476829b1e6a2e5a64d6c0eec7`에서 서명 설치형 `087cad916f2250df18c9c6b5d92713b5326c0e56f18f16053a2b317566d9f285`, 포터블 `7104e2dbd23d4d819d13062b8ef9f5aeb6ec50619a10c0b358a33fe1d0079fe0`을 생성했다.
- [GitHub v0.4.3](https://github.com/rubystarashe/nogirem/releases/tag/v0.4.3)의 자산 10개를 대치하고 역할 label을 복원했다. 공개 재다운로드 10/10이 후보 hash와 일치하고 공개 파일 입력 Ed25519·Electron bridge 재검증이 통과했다.
- 실제 제보 PC의 백신 격리·게임 폴더 ACL 적용은 NOT_RUN이며, 동일 0.4.3 사용자는 대치본을 수동 재설치해야 할 수 있다. source·release 기록은 `545ca42010917930877e4120eb33b6334f0cd9c3`까지 origin/main에 push됐다.
- 0.4.3은 affinity status의 PID·프로세스 시작 시각을 함께 검증하고, runtime polling 오류 뒤 상태 조회를 계속하며, 게임 미실행 대기 중에도 부스트 중단을 허용한다.
- 살아 있는 0.4.2 affinity helper는 control로 종료하고 동일 process instance의 종료를 확인한 뒤 0.4.3 helper로 교체한다.
- 터보키 helper 0.1.8은 정본의 `Mabinogi_Test` 폴더를 허용하며 사용자 쓰기 runtime 경로를 신뢰하지 않는다.
- Node 203/203, backend 60 PASS·2 declared ignore와 integration 2/2, turbo-key 14/14, desktop compile, package·Ed25519·release assembly가 통과했고 독립 review `APPROVED`다.
- 0.4.3 배포 후보 SHA-256은 설치형 `0eed1ea9...`, 포터블 `91f8415f...`, turbo-key `5b1e7952...`다. full desktop smoke는 `report.json` 생성 전 종료되어 실제 UI·테스트 서버 수동 QA와 함께 미검증으로 남는다.
- package source commit `5db679fd9c06cccaf9a29350d41f5e404528ad3b`; 원격 report 정리를 보존한 integration `d3c1c599045022fda8fa715dc45ecc06bde373e8`까지 origin/main에 push했다. [GitHub v0.4.3](https://github.com/rubystarashe/nogirem/releases/tag/v0.4.3)의 10개 자산은 공개 재다운로드해 후보 hash와 Ed25519 manifest·Electron bridge를 재검증했다.
- package source `cfdf43c68fd86f5db06bf127f6f06eab6035cc71`에서 설치형 `e51c7be021085892e5368cabfcd03b9e99e7e2885d61ca935a7e2f66b851a825`, 포터블 `a3b736b115c0912acb9745d1c18bad84bb6533defee6d784ac307c61abd6c439`을 생성해 GitHub v0.4.2 canonical·공개 alias와 signed manifest를 대치했다.
- 공개 CDN 실다운로드와 Ed25519 payload 재검증, Node 201개, release asset 3개, packaged native contract와 독립 review를 통과했다. 기존 0.4.2 사용자는 동일 버전을 자동 재수신하지 않으므로 로깅 대치본이 필요하면 수동 재설치해야 한다.
- affinity helper가 게임 감지·적용·종료 복원·시작 복구의 시작/완료 시각, 대상·변경·실패 수, mask와 ISLC·Process Lasso 충돌 상태를 `affinity/events.log`에 bounded JSONL로 기록한다.
- 진단 ZIP의 `diagnostics.json`은 기존 비정상 종료 외에 최근 14일 WHEA, Display, NVIDIA·AMD·DxgKrnl, WER LiveKernelEvent를 구분해 포함한다.
- backend 58개 통과·2개 환경 의존 ignore, desktop compile, Node 신뢰성 21개, Windows 이벤트 수집 실실행과 변경 파일 lint를 통과했다. 실제 하드 프리즈 후 생성된 신규 진단은 아직 없다.
- v0.4.2에 권장 설치형 `nogirem-setup-0.4.2.exe`와 포터블 `nogirem-portable-0.4.2.exe`를 추가하고 release 본문·파일명으로 직접 다운로드를 유도했다.
- 기존 Rust 자동 업데이트용 `nogirem-dioxus-*`와 signed manifests는 호환성을 위해 유지하며 `자동 업데이트 전용` label을 지정했다.
- update feed·blockmap·터보 키 helper는 `내부용` label로 역할을 낮춰 표시하고 사용자용 두 EXE만 파일명을 그대로 노출한다.
- Electron 전환본은 `nogirem-legacy-electron-migration-0.3.19.exe`로 바꾸고 latest.yml·sha512·blockmap을 공개 검증한 뒤 구 이름 자산을 제거했다.
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
- 채널 핑 오버레이 외의 화면 복제·스킬 슬롯 오버레이는 별도 요구사항과 권한 없이는 구현하지 않는다.
- Smart App Control 활성만으로 DXVK 설치를 막거나 DLL을 자동 삭제하지 않는다.
- 의미 있는 변경 후 이 파일을 갱신하고 설명 본문이 포함된 semantic commit을 만든다.

## Pending Tasks
1. 실패한 0.3.18에는 0.3.19, 0.4.0에는 수정된 0.4.1을 1회 수동 설치하도록 안내하고 성공 여부를 확인한다.
2. 이전 설치형·포터블 버전에서 0.4.1 업데이트 성공과 강제 실패 롤백을 종단간 검증한다.
3. 기존 0.4.1 포터블 사용자에게 수정 EXE를 1회 직접 내려받도록 안내하고 실제 성공 여부를 확인한다.
4. 0.4.x 무응답 사용자에게 보호 bootstrap 로그와 startup 로그를 받아 WebView 이전·이후 실패 단계를 판별한다.
5. 실제 앱에서 고해상도 휠과 일반 휠의 스크롤 감각을 확인한다.
6. 채널 핑 오버레이를 실제 마비노기 창과 여러 DPI에서 수동 검증하고 이후 배포 요청 시 release·공개 다운로드를 검증한다.

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
- 채널 핑 source·로컬 signed package는 완료했지만 실제 게임 수동 QA와 설치·제거 종단간·release는 완료하지 않았다.
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
