# 0.4.5 백신 오진 완화

- record_id: `RUN-20261006-ANTIVIRUS-FALSE-POSITIVE-045`
- run_id: `20261006-antivirus-false-positive-045`
- checkpoint_id: `CP-ANTIVIRUS-FALSE-POSITIVE-045`
- owner: `Cursor Agent`
- created_at: `2026-10-06T04:13:00Z`
- updated_at: `2026-10-06T09:26:10Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `9ce107f97cdbafad0c228777bb49b84d9256d059`
- request: `유료 Authenticode 코드 서명 없이 백신 오진 완화 개선 진행`
- objective: 신규 설치의 불필요한 임시 PowerShell 실행과 우회 정책을 제거하고, 설치형·포터블 PE 식별 정보와 분석 가능한 압축 구성을 추가한다.
- scope: `desktop/installer.nsi`, `desktop/portable.nsi`, 설치·포터블 계약 테스트, canonical packaging contract, handoff와 검증 기록.
- exclusions: Authenticode 구매·서명, 기존 공개 v0.4.5 자산 교체, push·release·배포, 네이티브 helper 재설계, 제품 기능 변경.
- risk: `MEDIUM — 설치·업데이트·제거 launcher 동작을 변경하지만 신규 설치에만 불필요한 종료 절차를 생략하고 기존 설치 경로는 보존함`
- authorization: `사용자가 코드 서명 외 개선 구현을 명시적으로 요청함`
- roles: `Coordinator=DEV=QA=Reviewer=Feature owner=Architecture owner=Documentation maintainer=Cursor Agent; SELF_REVIEW`
- dev_state: `DONE`
- qa_state: `COMPLETE_PASS`
- review_verdict: `APPROVED`
- stop_condition: 설치형·포터블 계약 검사와 실제 NSIS package가 통과하고 크기 제한·PE 메타데이터·PowerShell 제한을 확인하거나 정확한 blocker를 기록한다.
- feature_impact: `none — 제품 기능과 사용자 흐름은 유지하고 배포 wrapper의 평판·분석 가능성만 개선`
- feature_map: `no_change — 기존 기능의 observable behavior와 lifecycle은 바뀌지 않음`
- architecture_impact: `package security boundary — 신규 설치에서 stop script 미실행, ExecutionPolicy 우회 제거, non-solid LZMA와 PE version resource 적용`
- architecture_contract: `updated — revision 20에 package writer의 오진 완화 제약과 검증 근거 추가`

## Plan

1. 공개 0.4.5 설치형·포터블과 포함 helper의 서명·VirusTotal 탐지·sandbox 동작을 source와 대조한다.
2. 신규 설치에서 기존 파일이 없으면 종료 PowerShell을 생략하고 기존 설치·제거 경로만 유지한다.
3. 배포 NSIS 명령에서 `ExecutionPolicy Bypass`를 제거한다.
4. 설치형·포터블에 제품·파일 version resource를 추가하고 solid archive를 non-solid LZMA로 바꾼다.
5. 계약 테스트와 실제 미서명 package를 실행해 컴파일, 크기, 메타데이터와 payload 계약을 검증한다.
6. SELF_REVIEW, canonical contract, handoff와 최종 disposition을 기록하고 소유 파일만 checkpoint commit한다.

## Baseline evidence

- public installer: SHA-256 `aada494d6321bbcfd8875a2e4e38c32fc571ba0ea20a49399c88b0fc163308b7`, VirusTotal `4/71`
- public portable: SHA-256 `5269af6ca82b8f31e80decbe783665ae2ff3f1513c1caceccdbec45dc6c1bef8`, VirusTotal `6/71`
- Authenticode: 설치형·포터블과 추적 helper 4개 모두 `NotSigned`
- observed installer sandbox signal: 임시 NSIS directory의 `stop-installed-app.ps1`를 `-ExecutionPolicy Bypass`로 실행
- source assessment: update manifest의 Ed25519 서명은 payload 무결성용이며 Windows Authenticode 평판을 제공하지 않음

## Validation

- `PASS` — `node --test test/installer.test.mjs test/portable-release.test.mjs`: 14/14
- `PASS` — `npm test`: 208/208
- `PASS` — `npm run package:win:unsigned`: Rust release build, packaged native contract, Microsoft bootstrapper signature, NSIS `/WX`, 10MB installer budget
- `PASS` — final installer `9,698,406 bytes`, SHA-256 `212c9f44d74803c0848c75f25b8924c5079b9c87939be62e2e3e942d5feed5b4`
- `PASS` — final portable `7,925,246 bytes`, SHA-256 `a71af4bde58f9f86c7169133e3ab36b361490f6a61b4d0514cd2fe819d1a0e52`
- `PASS` — 두 outer PE의 product name, description, file/product version `0.4.5`, original filename 확인
- `PASS` — `git diff --check`
- `PASS` — 변경 파일 IDE diagnostics 0
- `PASS` — exact commit `af703e21f976d888cafe2ea1805a98b3b6816659` 격리 package 설치형 SHA-256 `7d13fb635220ed98155a1d820e2852b92c54ab3118cfeb204a638f516d8711a1`
- `PASS` — VirusTotal 재분석 `2/70`: CrowdStrike Falcon `Win/grayware_confidence_90% (D)`, Microsoft `Trojan:Win32/Wacatac.C!ml`
- `PASS` — 기존 공개 설치형 `4/71` 대비 Arctic Wolf `Unsafe`와 Elastic `Malicious (moderate Confidence)` 탐지 제거
- `NOT_RUN` — 실제 설치·업데이트·제거와 포터블 VirusTotal 재분석

## Findings and decisions

- 신규 설치 대상에 파일이 없으면 `stop-installed-app.ps1`를 추출하거나 PowerShell을 실행하지 않는다.
- 기존 설치 업데이트와 제거의 경로 제한 종료·예약 작업 정리는 유지하되 배포 명령의 `-ExecutionPolicy Bypass`를 제거한다.
- 설치형·포터블 outer PE에 제품명·설명·version·internal/original filename·copyright resource를 추가했다.
- non-solid LZMA는 분석 가능성을 높일 수 있어 시험했으나 64MB dictionary에서도 설치형이 `10,132,758 bytes`로 10MB 제한을 초과해 철회했다. solid LZMA와 그에 따른 overlay 휴리스틱 gap은 유지한다.
- 첫 package 시도는 `LegalCopyright` resource 누락 경고가 `/WX`에서 실패해 해당 standard key를 추가하고 재검증했다.

## SELF_REVIEW

- review_mode: `SELF_REVIEW`
- reviewed_scope: 신규·기존 설치 분기, update/uninstall 종료 호환성, PowerShell policy, NSIS version resource, compressor/size gate, 테스트와 canonical contract 정합성
- verdict: `APPROVED`
- mandatory_findings: `none`
- limitations: 동일 Cursor Agent가 구현과 검토를 수행했으며 독립 reviewer, 실제 Windows 설치 종단간 QA와 포터블 재분석은 없음

## Final disposition

- status: `COMPLETED`
- delivered: 코드 서명 없이 신규 설치의 불필요한 PowerShell 제거, policy bypass 제거, PE 식별 resource, 회귀 검사와 package gate
- feature_gate: `PASS — 제품 기능 변경 없음; feature map no_change`
- architecture_gate: `PASS — revision 20 갱신, package·크기·NSIS enforcement 통과`
- review_qa_gate: `PASS for automated source/package and installer VirusTotal comparison scope`
- target_authorization_gate: `PASS — local source·package만 변경/생성; push·release·배포 없음`
- remaining_risk: 설치형 탐지는 4개에서 2개로 감소했지만 미서명·관리자 권한·solid NSIS·포함 helper의 민감 동작 때문에 CrowdStrike grayware와 Microsoft Wacatac ML 탐지가 남음
- push_status: `NOT_RUN — 미승인`
- release_status: `NOT_RUN — 미승인`
- deployment_status: `NOT_RUN — 미승인`

## Accepted packaging disposition

- decided_at: `2026-10-06T09:25:00Z`
- authority: `user`
- decision: VirusTotal 최저 확인값 `2/70`을 만든 `af703e2`·`aa2e6a8` 계열 package 구조를 유지한다.
- effective_source: `811251a91de71119e0269528b5d4bf55abc835b4`
- included: 신규 설치 PowerShell 생략, `ExecutionPolicy Bypass` 미사용, outer PE metadata, solid LZMA, 기존 update/uninstall PowerShell 호환 경로.
- excluded: 별도 native cleanup helper 재도입과 AV 탐지 수를 위한 추가 구조 실험.
- feature_impact: `none — 제품 기능을 변경하지 않고 선택한 package 경계를 고정`
- feature_map: `no_change — capability·lifecycle 변화 없음`
- architecture_impact: `package cleanup boundary — aa2e6a8 구조를 승인된 현재 경로로 유지`
- architecture_contract: `no_change — canonical contract의 현재 PowerShell package 설명과 일치`
- remaining: rollback candidate의 exact VirusTotal 수치는 아직 확인되지 않았으며 `2/70`은 commit `af703e2` 격리 artifact 근거다.

## Native helper rollback

- checkpoint_id: `CP-ANTIVIRUS-NATIVE-ROLLBACK-045`
- requested_at: `2026-10-06T06:30:00Z`
- request: native helper 적용본이 `2/70 → 3/71`로 악화했으므로 이전 PowerShell cleanup 구현으로 되돌리고 0.4.4와 overlay 영향을 비교한다.
- decision: `ROLLBACK — AV 탐지 완화 목표를 달성하지 못했고 Bkav generic 탐지가 새로 발생함`
- dev_state: `DONE`
- qa_state: `COMPLETE_PASS`
- review_mode: `SELF_REVIEW`
- review_verdict: `APPROVED`
- feature_impact: `none — 설치·제거·포터블 cleanup을 직전 검증된 구현으로 복원하며 제품 capability는 변경하지 않음`
- feature_map: `no_change — observable 제품 기능과 lifecycle 변화 없음`
- architecture_impact: `package cleanup boundary — 별도 native helper를 제거하고 commit aa2e6a8의 NSIS·PowerShell 경계로 복원`
- architecture_contract: `no_change — concurrent game overlay owner의 canonical 문서를 수정하지 않고 기존 기록과 일치하는 구현으로 복원`

### Rollback evidence

- native helper crate·package wiring·계약 변경을 제거하고 `desktop/installer.nsi`, `desktop/portable.nsi`, `scripts/package-dioxus.mjs`, `scripts/stop-installed-app.ps1`, installer·portable tests와 `.gitignore`를 `aa2e6a8121720d8a9aa1e12a4949c14046929220` 상태로 복원했다.
- `PASS` — `node --test test/installer.test.mjs test/portable-release.test.mjs`: 14/14.
- `PASS` — `npm test`: 209/209.
- `PASS` — rollback package `npm run package:win:unsigned`, NSIS `/WX`, native contract와 10MB gate.
- `PASS` — rollback installer `9,701,745 bytes`, SHA-256 `361b62786781c28a0a97fdcc0266323c20d883a95be4e4b0af89410c1255dbe5`.
- `PASS` — rollback portable `7,929,476 bytes`, SHA-256 `2e1381b5624480e4d9c6fe68913840fd535ba6c81ee48d94ba45900d56a7ad14`.
- `PASS` — 공개 0.4.4 installer를 GitHub Release에서 다시 받아 SHA-256 `a92caa5878a12a7ba3697ab054978f39c2da9fed7a11bf093879d2bd5c61b368`, `9,542,395 bytes`로 release evidence와 일치함을 확인했다.
- `PASS` — Microsoft Defender local custom scan에서 공개 0.4.4와 rollback 0.4.5 모두 위협 없음.
- `BLOCKED` — 공개 0.4.4 hash는 VirusTotal에 등록되지 않았고 API는 인증 없이 HTTP 401이다. 현재 browser tool은 로컬 file input 설정을 지원하지 않아 사용자가 없는 동안 직접 업로드할 수 없음.

### Overlay causality assessment

- `af703e21f976d888cafe2ea1805a98b3b6816659` 개선본은 parent `9ce107f97cdbafad0c228777bb49b84d9256d059`까지의 game overlay capture·boss-bar detector를 포함한 상태에서 VirusTotal `2/70`이었다.
- `b905ec7fccfbe9342c87fca72340df30c7b1d430`은 missing-debuff alert와 package assets를 추가했지만 process termination·task cleanup API를 추가하지 않았다.
- `5d68346158193e8fbab2bb1e7e3cfc66b52a7433`에서 별도 unsigned helper와 Toolhelp/OpenProcess/TerminateProcess/schtasks 조합을 추가한 직후 Bkav `W32.Malware.9FC5E30`이 새로 발생했다.
- conclusion: `게임 overlay 자체보다 native cleanup helper가 Bkav 증가 원인일 가능성이 높음`. 다만 동일 엔진 시점의 overlay 포함 rollback package VirusTotal 업로드 전에는 인과를 확정하지 않는다.

### Rollback disposition

- status: `PARTIALLY_COMPLETED`
- completed: native helper source rollback, full automated regression, unsigned package, 0.4.4 artifact provenance와 local Defender 비교.
- pending: rollback 0.4.5와 공개 0.4.4 VirusTotal 업로드·동일 시점 결과 비교.
- blocker_owner: `VirusTotal file selection/API key=사용자`
- push_status: `NOT_RUN — 미승인`
- release_status: `NOT_RUN — 미승인`
- deployment_status: `NOT_RUN — 미승인`

## Concurrent work

- out_of_scope_dirty_changes: 진행 중인 `20261006-debuff-missing-overlay-045`의 source·asset·test 변경 전체
- owned_files: `desktop/installer.nsi`, `desktop/portable.nsi`, `test/installer.test.mjs`, `test/portable-release.test.mjs`, 이 run 기록과 packaging contract/handoff의 해당 section
- commit_policy: 위 소유 파일만 명시적으로 stage하며 기존 index와 디버프 작업을 보존한다.

## Approved native continuation

- checkpoint_id: `CP-ANTIVIRUS-NATIVE-CLEANUP-045`
- approved_at: `2026-10-06T05:38:00Z`
- request: 설치형·제거형 embedded `stop-installed-app.ps1`와 포터블 inline PowerShell 검사를 Rust/Win32 helper로 교체한다.
- authorization: 사용자가 PowerShell 제거·동등 동작 회귀·package·VirusTotal 재검사 순서를 승인했다.
- scope: `native/installer-cleanup`, installer·portable NSIS, package 조립, 계약·프로세스 회귀 검사, 기존 script 삭제.
- risk: `MEDIUM-HIGH — 관리자 helper가 설치 폴더 프로세스와 updater 및 시작 예약 작업을 정리하므로 경로·process identity·task ownership 경계가 중요함`
- roles: `Coordinator=DEV=QA=Feature owner=Architecture owner=Documentation maintainer=Cursor Agent; Reviewer=independent generalPurpose agent`
- dev_state: `READY_FOR_QA`
- qa_state: `IN_PROGRESS — automated helper/package checks passed; actual lifecycle and VirusTotal pending`
- review_state: `APPROVED for current source review target`
- feature_impact: `none — 설치·업데이트·제거와 포터블 캐시의 observable 동작을 보존하는 내부 구현 교체`
- feature_map: `no_change — 제품 capability·lifecycle·acceptance scope는 변하지 않음`
- architecture_impact: `package cleanup boundary — 임시 PowerShell 대신 별도 최소 Rust/Win32 helper가 process·task cleanup을 소유`
- architecture_contract: `blocked — canonical contract의 PowerShell 설명 갱신이 필요하지만 진행 중인 game overlay 작업이 같은 문서를 수정 중이며 해당 owner와 병합 전임`

### Implementation

- `nogirem-installer-cleanup.exe`는 Toolhelp snapshot과 열린 process handle로 설치 경로·updater 경로를 검사한다.
- 현재 helper·호출 installer/uninstaller와 검증된 명시 updater 또는 구버전 installer의 부모 updater를 보존한다.
- `%APPDATA%\마비노기 렘 부스터\instance\installer-close-request`를 반복 기록하고 30초 뒤 설치 경로에 남은 process만 강제 종료한다.
- 제거 시 고정 예약 작업 XML의 `Command`가 해당 설치의 `nogirem.exe`와 정확히 일치할 때만 `schtasks.exe`로 제거한다.
- task file과 `schtasks.exe`는 환경변수가 아닌 `GetWindowsDirectoryW`·`GetSystemDirectoryW`로 찾는다.
- 포터블 cache check는 cache의 정확한 `nogirem.exe` process path만 사용 중으로 판정하고 exit code 2를 유지한다.
- helper에는 제품명·설명·0.4.5 version·original filename·copyright PE resource를 넣고 기존 PowerShell source를 package와 저장소에서 제거했다.

### QA and evidence

- `PASS` — `cargo test --locked --manifest-path native/installer-cleanup/Cargo.toml`: 8 PASS, test process fixture 3 declared ignored.
- `PASS` — 실제 process 시나리오: 강제 종료, 대기 중 신규 process 재탐색, 정상 종료 요청, stale updater 종료, 명시 updater 보존, 0.4.1 부모 updater 보존, 포터블 exact/negative path.
- `PASS` — `node --test test/installer.test.mjs test/portable-release.test.mjs`: 11/11.
- `PASS` — `npm test`: 206/206.
- `PASS` — `npm run package:win:unsigned`: Rust release helper, native service contract, Microsoft bootstrapper signature, NSIS `/WX`, 10MB gate.
- `PASS` — installer `9,932,829 bytes`, SHA-256 `f1ae91bb416a832dc2408662b89da3016cdf6ea4b87109468892fb009206d298`.
- `PASS` — portable `8,027,656 bytes`, SHA-256 `524f5c06283f691ada1ff0e2d6d5398c096d620a95e39a300941061fde0d46ff`.
- `PASS` — helper `297,984 bytes`, ProductName `마비노기 렘 부스터`, FileDescription `마비노기 렘 부스터 설치 정리 도구`, FileVersion `0.4.5`.
- `PASS` — installer·portable outer PE product·description·version resource.
- `PASS` — IDE diagnostics 0, owned source `git diff --check`.
- `NOT_RUN` — 실제 설치·업데이트·제거: 임시 `/D=` 설치 시도의 reliable process exit와 target 생성이 확인되지 않아 실제 설치 경로를 보호하기 위해 중단했다. 기존 `C:\Program Files\Nogirem\nogirem.exe` hash와 timestamp는 final package와 달라 변경되지 않았음을 확인했다.
- `NOT_RUN` — signed updater rollback: unsigned candidate에는 검증 가능한 update envelope가 없음.
- `PASS` — commit `5d68346158193e8fbab2bb1e7e3cfc66b52a7433` package installer VirusTotal 재분석 `3/71`.
- `PASS` — 비정상 판정: Bkav `W32.Malware.9FC5E30`, CrowdStrike Falcon `Win/grayware_confidence_90% (D)`, Microsoft `Trojan:Win32/Wacatac.B!ml`.
- `PASS` — 임베디드 PowerShell 제거 여부와 별개로 이전 PE metadata 개선본 `2/70`보다 Bkav generic 탐지 하나가 늘었고 CrowdStrike·Microsoft는 남아, 탐지 수 추가 개선은 확인되지 않음.

### Independent review exchange

- round_id: `REV-ANTIVIRUS-NATIVE-045-R1`
- review_mode: `INDEPENDENT_REVIEW`
- reviewer: `generalPurpose agent 90ac6910-35a8-42e8-875f-7610740a0399`
- initial_verdict: `CHANGES_REQUESTED`
- findings: `ISSUE-INSTALLER-001 task action ownership`, `ISSUE-UPDATER-003 PID reuse`, `ISSUE-TEST-005 missing scenarios`, `ISSUE-INSTALLER-006 untrusted SystemRoot`; 모두 mandatory.
- dev_response: task XML action 검증·실패 처리, caller/updater handle lifetime 유지, process scenario 확대, trusted Win32 directory API 적용.
- rereview_verdict: `APPROVED`
- resolved: `ISSUE-INSTALLER-001`, `ISSUE-UPDATER-003`, `ISSUE-TEST-005`, `ISSUE-INSTALLER-006`.
- deferred_non_regressions: `ISSUE-PORTABLE-002 mutex 해제 후 app 시작`, `ISSUE-INSTALLER-004 quiet period 뒤 file copy 전 재실행 가능성`; 기존 구현과 동일하며 별도 authorization이 필요한 잔여 위험.
- approval_freshness: 현재 source 변경 뒤 Node·Rust·package를 재실행했으며 final package 결과가 review source 논리를 변경하지 않음.

### Continuation handoff

- status: `PARTIALLY_COMPLETED`
- current_source_base: `aa2e6a8121720d8a9aa1e12a4949c14046929220`
- current_source_target: `5d68346158193e8fbab2bb1e7e3cfc66b52a7433`
- current_execution_target: `release/dioxus-0.4.5-2026-10-06T06-00-25-179Z`
- completed: native helper 구현, PowerShell 제거, regression·package·size·PE resource·independent review.
- pending: 실제 lifecycle QA, canonical architecture contract와 `.handoff/cursor.md` 병합.
- blocker_owner: `shared documentation ownership=game overlay coordinator; actual lifecycle executor=coordinator`
- next_authorized_action: 공유 문서 소유권이 해소되면 package contract/handoff를 갱신하고, 격리된 Windows 대상에서 실제 install/update/uninstall/rollback QA를 수행한다.
- push_status: `NOT_RUN — 미승인`
- release_status: `NOT_RUN — 미승인`
- deployment_status: `NOT_RUN — 미승인`
