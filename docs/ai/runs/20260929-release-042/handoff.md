# 0.4.2 인계

- record_id: `HANDOFF-RELEASE-042-01`
- run_id: `20260929-release-042`
- checkpoint_id: `CP-RELEASE-042-01`
- owner: `cursor-agent-92251f36`
- updated_at: `2026-09-29T14:06:00Z`
- status: `COMPLETED`

## 상태와 target

- DEV: `DONE`
- QA: `COMPLETE_PASS` for mandatory release scope
- review: `APPROVED`, independent, `ISSUE-UPDATER-001 RESOLVED`
- source: `86ca9261ab763e57477c7655d3f303b256d29bd8`
- remote tag: `v0.4.2` peeled to source commit
- release: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.2`
- execution target: Windows 10.0.26200 x64, signed installer·portable and public GitHub download

## 완료

- Electron·Rust FastPing route query가 기본 경로 부재를 구조화된 연결 없음으로 반환한다.
- backend가 main UI open에 성공한 직후 건강 상태를 기록하고 시작 animation·worker·PowerShell 조회와 분리했다.
- NSIS PowerShell을 hidden runner로 실행하고 포터블 정상 무프로세스의 PowerShell `exit 1`을 제거했다.
- 직접 설치·제거는 stale updater helper를 종료하고 설치 폴더 process를 반복 재탐색한다.
- 새 updater PID와 이전 0.4.1 실제 부모 updater를 모두 검증해 정상 update helper 하나만 보존한다.
- 정본 feature map과 architecture contract를 생성하고 wiki index와 `.handoff/cursor.md`에 연결했다.

## 검사와 evidence

- Node 198/198 `PASS`
- Rust backend 56 executed, 1 declared live-network ignore, all executed `PASS`
- Rust desktop check `PASS`
- focused startup/update UI smoke `PASS`
- signed package and NSIS compile `PASS`
- public manifest Ed25519 verification and installer/portable full download SHA-256 `PASS`
- full app smoke: `FAIL`, 기존 `Advanced structure failed: null` harness gap. 관련 focused smoke는 통과했다.

## 산출물

- installer SHA-256: `303a9008f2bdd4a42c93cab7e9594e47f953617ab6d2d6a31ff4859f0d92ee04`
- portable SHA-256: `fd956491fe940c9c02c8faa5782c03b0afbe395f24c19ea7f46dad0e15c57ad7`
- local bundle: `release/ready-v0.4.2-2026-09-29T13-57-45-990Z`

## 남은 항목과 재개

- 실제 사용자 장치에서 0.4.1→0.4.2 앱 내 update와 인터넷 단절·복구 UI 수동 확인은 남아 있다.
- 기존 full UI smoke의 advanced structure harness failure는 이번 변경 범위 밖의 알려진 gap이다.
- blocker: 없음
- next authorized action: 사용자 진단이 들어오면 해당 target과 runtime provenance를 확인한다. 별도 요청 없이 추가 배포나 자산 교체는 하지 않는다.
- safe resume: 저장소에서 `git show 86ca9261ab763e57477c7655d3f303b256d29bd8`와 이 run의 `qa.md`를 먼저 확인한다.

## 영향

- feature_impact: `FEAT-NOGIREM-NETWORK-FASTPING`, `FEAT-NOGIREM-UPDATE-LIFECYCLE`
- feature_map: `updated — docs/ai/wiki/feature-map.md`
- architecture_impact: `네트워크 writer, backend 준비 신호, updater/installer/helper process 경계`
- architecture_contract: `updated — docs/ai/wiki/agent-friendly-architecture.md`

## CP-RELEASE-042-02 대치 배포 인계

- updated_at: `2026-09-29T18:10:00Z`
- status: `IN_PROGRESS`
- DEV: `READY_FOR_QA`
- QA: `IN_PROGRESS`
- review: `APPROVED`, independent, `ISSUE-INTEGRITY-001/002 RESOLVED`
- source target: base `3902f13bf9a7b110df6a516150e466b470b78e4a`, `Cargo.toml a5bc58de...4593d`, `update_install.rs 9f69d01e...c8a42`
- completed: PowerShell SDDL substring 검사를 `GetNamedSecurityInfoW(LABEL_SECURITY_INFORMATION)`와 검증된 mandatory ACE 정책으로 교체하고 `icacls /C`가 실패를 숨기지 않게 했다.
- checks: Node 198/198 `PASS`; Rust backend 55/55 실행, 2개 명시 ignore `PASS`; desktop check `PASS`; independent rereview `APPROVED`.
- limitation: 실제 High label 경로는 Cursor sandbox의 `icacls` 오류 1299로 `BLOCKED`; 합성 High/System/Medium/policy/authority/RID 회귀와 API 계약을 검증했다.
- blocker: 없음. 남은 작업은 실서명 패키징, 공개 자산 대치, 실다운로드 검증과 최종 문서화다.
- next authorized action: 현재 target을 로컬 커밋한 뒤 `npm run package:win`으로 0.4.2 자산을 만들고 GitHub v0.4.2 자산을 대치한다.
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — 정상 High integrity 작업 오거부 제거`
- feature_map: `no_change — 구현 checkpoint 진행 중이며 배포 증거 뒤 갱신 예정`
- architecture_impact: `updater 작업 파일 mandatory-integrity 검증 경계`
- architecture_contract: `no_change — 배포 증거 뒤 정본 freshness 갱신 예정`

## CP-RELEASE-042-02 완료

- updated_at: `2026-09-29T18:16:00Z`
- status: `COMPLETED`
- DEV: `DONE`
- QA: `COMPLETE_PASS for mandatory automated replacement scope`
- review: `APPROVED`, fresh for source `3cb35d86bf8967a86303b9440754445d9d338443`
- release: [GitHub v0.4.2](https://github.com/rubystarashe/nogirem/releases/tag/v0.4.2) 네 자산 대치 완료.
- installer: `9460244 bytes`, SHA-256 `8ed390dbcc809fbaf3fbe84a591b1bda6ebb7974f7079f503030d39d2caa95c0`
- portable: `7693818 bytes`, SHA-256 `128d4a3d0a381cbf2ad4bee183fb4d4a64dc0ae80635a9cfed2e4f37277d2b0c`
- public `update.json`: `5c08dbbe0d9111b486ce98a5a46e862608c59edb579f9365769fcfe8afeba889`
- public `portable-update.json`: `1e33a650c51fcbf9d39f1c1186e895ba1b93edbf8abe6cb6e47e942eaa6583f7`
- external limitation: 실제 관리자 토큰의 High label 종단간 QA와 이미 막힌 0.4.1 사용자의 수동 설치 성공 확인이 남아 있다.
- safe resume: 위 사용자는 v0.4.2 설치형을 한 번 직접 실행하고, 성공 후 다음 업데이트에서 앱 내 경로를 확인한다.
- commit: `3cb35d86bf8967a86303b9440754445d9d338443`; push/tag rewrite/deployment activation은 수행하지 않았다.
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE`
- feature_map: `updated — revision 2`
- architecture_impact: `native mandatory-label trust boundary`
- architecture_contract: `updated — revision 2`

## CP-RELEASE-042-03 파일명 전환 인계

- updated_at: `2026-09-30T10:06:00Z`
- status: `IN_PROGRESS`
- DEV: `READY_FOR_QA`
- QA: `IN_PROGRESS`
- review: `APPROVED`, independent, `ISSUE-ASSET-001~004 RESOLVED`
- source target: base `57075bd7582976d39cabfe60341671a91a5afb2a` + reviewed uncommitted diff
- compatibility: 배포된 0.4.x verifier가 `nogirem-dioxus-*`를 요구하므로 해당 자산과 signed manifests는 자동 업데이트 전용으로 유지한다. 짧은 설치형·포터블 이름은 byte-identical 수동 다운로드 alias다.
- Electron: `latest.yml`이 새 `nogirem-legacy-electron-migration-0.3.19.exe`를 가리키며 sha512와 blockmap을 검증한다.
- checks: focused 5/5 `PASS`; actual signed bundle assembly `PASS`; full Node 201/201 `PASS`.
- remaining: source commit, 공개 alias·legacy feed 업로드, 자산 label·release 안내, 구 legacy 이름 제거, 실다운로드 검증.
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE`
- feature_map: `blocked — 배포 target 확정 뒤 revision 3 갱신`
- architecture_impact: `release-assets 조립·alias·Electron feed 무결성`
- architecture_contract: `blocked — 배포 target 확정 뒤 revision 3 갱신`
