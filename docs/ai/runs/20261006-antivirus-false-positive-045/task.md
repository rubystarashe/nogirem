# 0.4.5 백신 오진 완화

- record_id: `RUN-20261006-ANTIVIRUS-FALSE-POSITIVE-045`
- run_id: `20261006-antivirus-false-positive-045`
- checkpoint_id: `CP-ANTIVIRUS-FALSE-POSITIVE-045`
- owner: `Cursor Agent`
- created_at: `2026-10-06T04:13:00Z`
- updated_at: `2026-10-06T04:26:00Z`
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
- `NOT_RUN` — 실제 설치·업데이트·제거와 새 artifact VirusTotal 업로드·재분석

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
- limitations: 동일 Cursor Agent가 구현과 검토를 수행했으며 독립 reviewer와 실제 Windows 설치 종단간 QA는 없음

## Final disposition

- status: `COMPLETED`
- delivered: 코드 서명 없이 신규 설치의 불필요한 PowerShell 제거, policy bypass 제거, PE 식별 resource, 회귀 검사와 package gate
- feature_gate: `PASS — 제품 기능 변경 없음; feature map no_change`
- architecture_gate: `PASS — revision 20 갱신, package·크기·NSIS enforcement 통과`
- review_qa_gate: `PASS for automated source/package scope`
- target_authorization_gate: `PASS — local source·package만 변경/생성; push·release·배포 없음`
- remaining_risk: 미서명·관리자 권한·solid NSIS·포함 helper의 민감 동작 때문에 탐지가 완전히 사라진다는 보장은 없으며 새 artifact 재분석 필요
- push_status: `NOT_RUN — 미승인`
- release_status: `NOT_RUN — 미승인`
- deployment_status: `NOT_RUN — 미승인`

## Concurrent work

- out_of_scope_dirty_changes: 진행 중인 `20261006-debuff-missing-overlay-045`의 source·asset·test 변경 전체
- owned_files: `desktop/installer.nsi`, `desktop/portable.nsi`, `test/installer.test.mjs`, `test/portable-release.test.mjs`, 이 run 기록과 packaging contract/handoff의 해당 section
- commit_policy: 위 소유 파일만 명시적으로 stage하며 기존 index와 디버프 작업을 보존한다.
