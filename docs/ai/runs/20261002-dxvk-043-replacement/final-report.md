# 0.4.3 DXVK 대치 최종 보고

- status: `COMPLETED`
- run_id: `20261002-dxvk-043-replacement`
- checkpoint_id: `CP-DXVK-043-REPLACE`
- owner: `Cursor Agent`
- created_at: `2026-10-02T15:30:00Z`
- source_base: `8f365e1f0b4566f0c41da87856e9a290b59a0d28`
- source_commit: `567ab935aad7d69476829b1e6a2e5a64d6c0eec7`
- source_manifest: `SOURCE-MANIFEST-DXVK-043-R4`, SHA-256 `8ad94541aa2daef8b158ac3b309cd93a8e6238422d99b02c708f9762ff75efba`
- feature_impact: `FEAT-NOGIREM-DXVK — game path 검증, handle 기반 DLL 교체, backup rollback과 오류 안내`
- feature_map: `updated — docs/ai/wiki/feature-map.md revision 5`
- architecture_impact: `canonical/reparse trust boundary, Win32 handle identity·rename, 외부 deployment 검증까지 유지되는 backup transaction`
- architecture_contract: `updated — docs/ai/wiki/agent-friendly-architecture.md revision 5`

## 제공 범위

- 실제 `Client.exe`, 허용 폴더·launcher, canonical parent와 모든 상위 reparse를 검증한다.
- 다운로드 뒤 게임 실행·경로 변경을 staged file 교체 직전에 다시 확인한다.
- staged·기존 파일과 디렉터리의 Win32 identity를 확인하고 staged handle의 `FileRenameInfo`로 교체한다.
- 내부·외부 검증 실패 시 기존 backup을 복원하며 복구 실패 시 검증된 backup을 보존한다. 원본이 없던 실패 target은 제거한다.
- Windows file not found, access denied, sharing violation을 작업 단계와 해결 안내가 있는 한국어 오류로 구분한다.
- GitHub `v0.4.3`의 설치형·포터블·manifest와 호환 자산 10개를 대치했다.

## 검증·리뷰

- automated-test-passing: backend `71 PASS`, `2 declared ignore`, integration `2/2`; DXVK `14/14`; Node `203/203`; desktop locked check.
- package: signed installer·portable, NSIS `/WX`, native contract, WebView2 서명, Ed25519 manifest와 release assembly `10/10` 통과.
- public: 공개 자산 `10/10` 후보 SHA-256 일치; 공개 다운로드를 입력으로 Ed25519·Electron bridge 재검증 통과.
- review_mode: `INDEPENDENT_REVIEW`
- reviewer: `cursor subagent 3a60e26f-1641-4d5d-ac1b-3192372be75c`
- verdict: `APPROVED round 4`
- findings: `ISSUE-DXVK-043-001~006 RESOLVED`, unresolved mandatory finding `0`, approval stale `false`.
- manually exercised: 실제 게임·제보 PC 시나리오는 `NOT_RUN`.

## 배포·게이트

- release: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.3`
- installer SHA-256: `087cad916f2250df18c9c6b5d92713b5326c0e56f18f16053a2b317566d9f285`
- portable SHA-256: `7104e2dbd23d4d819d13062b8ef9f5aeb6ec50619a10c0b358a33fe1d0079fe0`
- feature gate: `PASS`
- architecture gate: `PASS`
- review and QA gate: `PASS`
- integration gate: `PASS — 단일 저장소, Electron bridge·turbo helper 호환 자산 보존`
- target and authorization gate: `PASS`
- commit: `COMPLETED`
- push: `COMPLETED — origin/main through 545ca42010917930877e4120eb33b6334f0cd9c3`
- deployment: `COMPLETED — 기존 v0.4.3 자산 대치, tag rewrite 없음`
- activation: `NOT_RUN — 사용자 장치 설치·실행 안 함`

## 남은 위험

- 제보 사용자 PC의 백신 격리, 실제 게임 폴더 ACL과 의도적 junction 경쟁은 수동 재현하지 않았다.
- 동일 버전 대치이므로 이미 0.4.3을 설치한 사용자는 자동 버전 증가를 감지하지 못할 수 있으며 수동 재설치가 필요할 수 있다.
