# 0.4.3 DXVK 파일 오류 수정·대치 배포

- run_id: `20261002-dxvk-043-replacement`
- checkpoint_id: `CP-DXVK-043-REPLACE`
- owner: `Cursor Agent`
- created_at: `2026-10-02T14:33:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `8f365e1f0b4566f0c41da87856e9a290b59a0d28`
- roles: `Cursor Agent = Coordinator + DEV + QA + Documentation maintainer`, `independent reviewer = 배포 전 지정`
- feature_impact: `FEAT-NOGIREM-DXVK — 다운로드한 DLL 저장, 게임 경로 확인, 게임 폴더 적용과 실패 안내`
- feature_map: `updated — DXVK 기능 identity·시나리오·writer·검증 범위를 추가`
- architecture_impact: `DXVK 저장소와 게임 폴더의 단일 writer, Windows 원자 교체, game path와 실행 상태 재검증`
- architecture_contract: `updated — DXVK download·file transaction 경계를 추가`

## 요청·권한·위험

- 원 요청: Vulkan 다운로드 시 일부 사용자에게 `지정된 파일을 찾을 수 없습니다. (os error 2)`가 표시되는 문제를 개선하고 GitHub `v0.4.3` 자산을 대치한다.
- 포함: Rust DXVK manager·file transaction, 회귀 테스트, 변경 기록·정본 문서, 서명 설치형·포터블 패키징, 기존 v0.4.3 자산 대치, 공개 검증, source push.
- 제외: Windows 보안·백신 설정 자동 변경, 게임 실행 중 강제 DLL 교체, tag/history rewrite, 사용자 PC 원격 조작.
- risk: `HIGH — 사용자 게임 폴더 DLL writer와 서명 자동 업데이트 payload를 변경하고 공개 자산을 대치함`
- low_risk: `not_applicable — durable file writer와 배포 payload가 바뀌므로 저위험 변경이 아님`

## 계획·상태

- DEV: `DONE`
- QA: `COMPLETE_PASS`
- review: `APPROVED round 4`
- release: `COMPLETED`

1. `COMPLETED` — 저장·적용 단계의 raw OS 오류 원인과 잘못된 게임 경로 선검증 누락 확인
2. `COMPLETED` — game path 선·후 검증, same-directory 임시 파일, Windows replace, 임시·최종 hash 검증과 단계별 오류 구현
3. `COMPLETED` — 경로 누락·기존 DLL 교체 회귀와 전체 자동 검사
4. `COMPLETED` — 4개 round에서 mandatory finding 6개 수정, exact source target 독립 review 승인
5. `COMPLETED` — source checkpoint commit, signed package 생성·검증
6. `COMPLETED` — GitHub v0.4.3 자산 대치·공개 검증·최종 handoff; push만 마지막 기록 commit 뒤 수행

## 수용 기준·시나리오

- `DXVK-001`: 기존 `d3d9_dxvk.dll`이 있어도 Windows replace API로 교체하고 최종 hash가 입력과 일치한다.
- `DXVK-002`: 전용 저장 하위 폴더가 없으면 생성한 뒤 DLL을 저장한다.
- `DXVK-003`: 실제 `Client.exe`, 허용 마비노기 폴더 또는 launcher와 canonical·reparse 검사를 통과하지 못하면 다운로드 전에 중단한다.
- `DXVK-004`: 다운로드 중 게임 경로가 달라지거나 게임이 실행되면 교체 직전 guard에서 DLL을 적용하지 않는다.
- `DXVK-005`: 임시·최종 DLL이 사라진 `NotFound`는 Windows 보안·백신 격리 가능성을, access denied·sharing violation은 보안 차단·권한·파일 사용을 안내한다.
- `DXVK-006`: release archive·DLL·staged file·final file 무결성 검증이 모두 유지된다.
- `DXVK-007`: 교체 전 실패는 기존 DLL을 유지하고, 교체 후 검증 실패는 검증된 backup을 복원하며 임시 파일을 정리한다.
- `REL-043-DXVK-001`: backend·Node·desktop compile, package contract, Ed25519 manifest, 공개 payload digest가 정확한 target에서 통과한다.

## 현재 QA

- `PASS` — round 4 backend `71 PASS`, `2 declared ignore`, integration `2/2`
- `PASS` — Node `203/203`
- `PASS` — desktop `cargo check --manifest-path desktop/Cargo.toml --locked`
- `PASS` — round 4 DXVK 전용 `14/14`, Node `203/203`, desktop locked check
- `PASS` — 변경 파일 IDE lint와 `git diff --check`
- `FAIL → FIXED` — 빈 최신 `REPORT.json`과 삭제된 과거 report ID를 결합한 오래된 test fixture를 현재 유효 응답 구조 검증으로 수정한 뒤 Node `203/203`
- `PASS` — signed installer·portable package, NSIS warning-as-error, native contract, WebView2 bootstrap 서명, Ed25519 manifest 생성·검증
- `PASS` — release assembly 10/10, canonical/alias byte identity, Electron 0.3.19 bridge·turbo-key 0.1.8 보존
- `PASS` — GitHub 공개 자산 10개 재다운로드 SHA-256 일치와 공개 파일 입력 Ed25519·Electron bridge 재검증
- `NOT_RUN` — 제보 사용자 PC의 보안 제품 격리와 실제 게임 폴더 적용

## 독립 review

- reviewer: `cursor subagent 3a60e26f-1641-4d5d-ac1b-3192372be75c`
- review_mode: `INDEPENDENT_REVIEW`
- round 1 verdict: `BLOCKED`
- round 2 verdict: `CHANGES_REQUESTED`; `ISSUE-DXVK-043-003~005 RESOLVED`
- round 3 verdict: `CHANGES_REQUESTED`; exact manifest 확인 완료
- round 4 verdict: `APPROVED`; unresolved mandatory finding `0`, 새 finding `0`
- `ISSUE-DXVK-043-001 READY_FOR_REREVIEW` — 허용 폴더·launcher, canonical·reparse 검증에 더해 디렉터리 volume/file-index identity를 경로와 syscall 직전에 비교하고 staged handle 자체의 `SetFileInformationByHandle`로 교체한다.
- `ISSUE-DXVK-043-002 READY_FOR_REREVIEW` — backup을 외부 deployment 검증까지 유지하며 `deployment()`의 I/O `Err`도 rollback한다. 복구 실패 시 backup 보존, 원본 없음 target 제거와 해당 회귀를 포함한다.
- `ISSUE-DXVK-043-003 READY_FOR_REREVIEW` — game active·canonical target 검사를 file replace 직전 guard로 이동했다.
- `ISSUE-DXVK-043-004 READY_FOR_REREVIEW` — Win32 2/3·5·32/33 오류 안내와 잠긴 target 보존, guard 실패 보존·cleanup 테스트를 추가했다.
- `ISSUE-DXVK-043-005 READY_FOR_REREVIEW` — 원본 REPORT 응답 수와 정규화 결과 수를 비교해 invalid-only 문서가 통과하지 못하게 했다.
- `ISSUE-DXVK-043-006 READY_FOR_REREVIEW` — `SOURCE-MANIFEST-DXVK-043-R4`에 source file·diff SHA-256과 생성 절차를 기록하고 정본 문서를 실제 handle rename·backup 보존 계약에 맞췄다.

## Round 4 source target

- manifest: `docs/ai/runs/20261002-dxvk-043-replacement/evidence/source-manifest.txt`
- manifest SHA-256: `8ad94541aa2daef8b158ac3b309cd93a8e6238422d99b02c708f9762ff75efba`
- base commit: `8f365e1f0b4566f0c41da87856e9a290b59a0d28`
- source diff SHA-256: `0ac7944b89344711a23fe48c5a581e8a8a41a8adf06878a87c4acce2273fad27`
- index: unchanged
- untracked source/test files: none

## 배포 후보와 공개 결과

- package source commit: `567ab935aad7d69476829b1e6a2e5a64d6c0eec7`
- build: `release/dioxus-0.4.3-2026-10-02T15-20-01-171Z`
- candidate: `release/ready-v0.4.3-2026-10-02T15-24-35-158Z`
- installer canonical/alias SHA-256: `087cad916f2250df18c9c6b5d92713b5326c0e56f18f16053a2b317566d9f285`
- portable canonical/alias SHA-256: `7104e2dbd23d4d819d13062b8ef9f5aeb6ec50619a10c0b358a33fe1d0079fe0`
- update manifest SHA-256: `9573b097cdf251e28e5f702850c53613686e7932f64fd977c618d2374033bc18`
- portable manifest SHA-256: `193e3b66005a77fddf80cf425d804cd503d1d35c9bca4bb2295c2442db8dc8`
- preserved turbo-key SHA-256: `5b1e79528b615cd02ca7a7ec59a45a0f0942e58130c8ac0bc6540fb4ccd78936`
- GitHub release: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.3`
- release state: `10 assets uploaded, draft=false, prerelease=false, 역할 label 복원 완료`
- public verification: `후보와 공개 10/10 SHA-256 일치; 공개 다운로드를 입력으로 prepare-release 재실행 PASS`

## 완료 게이트와 최종 상태

- overall: `COMPLETED`
- feature gate: `PASS — FEAT-NOGIREM-DXVK 기능 map과 exact source·behavior evidence 갱신`
- architecture gate: `PASS — canonical/reparse·Win32 handle identity·handle rename·backup rollback 경계와 enforcement 통과`
- review and QA gate: `PASS — independent APPROVED, ISSUE-DXVK-043-001~006 RESOLVED, 자동·package·공개 검증 통과`
- target and authorization gate: `PASS — source·release record commit과 package/public digest 식별, 기존 v0.4.3 tag rewrite 없음`
- commit: `source 567ab935aad7d69476829b1e6a2e5a64d6c0eec7, release record 545ca42010917930877e4120eb33b6334f0cd9c3`
- push: `COMPLETED — origin/main through 545ca42010917930877e4120eb33b6334f0cd9c3`
- deployment/activation: `GitHub v0.4.3 release assets replaced`; 사용자 장치 설치·실행은 `NOT_RUN`
- remaining risk: 실제 제보 PC의 AV 격리·게임 폴더 ACL에서 수동 재현하지 못했으며, 동일 0.4.3 사용자는 자동 버전 증가가 없어 대치본을 수동 재설치해야 할 수 있다.

## 검증 한계·중단 조건

- 자동 테스트는 temp 폴더의 기존 DLL 교체와 경로 검증을 실행했지만 실제 게임 설치 폴더 ACL·백신 격리는 재현하지 않았다.
- `NotFound`가 항상 보안 격리를 뜻하지 않으므로 메시지는 가능성으로만 안내한다.
- 독립 review mandatory finding, package·signature·digest 불일치, 공개 자산 개수·이름 불일치가 있으면 배포를 중단한다.
- commit/push/deployment: `NOT_RUN`
