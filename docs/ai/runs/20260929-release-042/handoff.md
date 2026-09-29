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
