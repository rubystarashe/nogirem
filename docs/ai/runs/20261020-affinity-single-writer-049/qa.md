# QA — affinity helper single writer 0.4.9

- record_id: `QA-AFFINITY-SINGLE-WRITER-049`
- run_id: `20261020-affinity-single-writer-049`
- checkpoint_id: `CP-AFFINITY-SINGLE-WRITER-049`
- owner: `cursor-agent-92251f36`
- QA_role: `DEV-as-QA`
- created_at: `2026-10-20T04:27:00Z`
- source_target: `uncommitted target based on 37a982642d0e44e56a3567acbbe06f849248751e`
- execution_target: `release/dioxus-0.4.9-2026-10-20T04-22-57-674Z`
- state: `COMPLETE_PASS`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — affinity helper 단일 writer와 시작 오류 진단 개선`
- feature_map: `updated — canonical revision 40, feature revision 17`
- architecture_impact: `helper lock exact process ownership과 startup status·stderr 경계`
- architecture_contract: `updated — revision 44`

## Scenario matrix

- `BOOST-016-A`, `PASS` — status가 없고 lock record가 오래돼도 exact live PID·시작 시각 owner의 lock을 탈취하지 않는다. Rust unit test.
- `BOOST-016-B`, `PASS` — lock drop ownership은 PID와 process 시작 시각이 모두 일치해야 한다. Rust unit test.
- `BOOST-016-C`, `PASS` — coordinator가 시작 전에 status를 삭제하지 않고 child stderr pipe를 보존하는 static contract.
- `BOOST-016-D`, `PASS` — packaged helper 두 개를 같은 격리 경로로 실행하면 첫 helper는 유지되고 두 번째는 exit 1과 `Helper already running`을 반환한다.
- `BOOST-016-E`, `PASS` — control stop 뒤 첫 helper가 exit 0, final status `running=false`, TEMP fixture cleanup.
- `VER-049`, `PASS` — package/Cargo/lock/app payload가 모두 `0.4.9`.
- `SIGN-049`, `PASS` — installer·portable Ed25519 signature, version, size와 SHA-256.
- `REG-BACKEND`, `PASS` — 130 PASS, 3 ignored, parity 2/2.
- `REG-DESKTOP`, `PASS` — 8/8.
- `REG-NODE`, `PASS` — 213/213.
- `REG-NATIVE`, `PASS` — memory worker, affinity worker, native network operation 3/3.
- `STRICT-CLIPPY`, `FAIL` — Rust 1.98 `-D warnings`에서 대상 외 기존 파일 35건의 신규 lint가 실패했다. 변경 파일 `storage.rs`, `boost.rs`, `memory.rs` finding은 0이며 compile·tests·package는 PASS.
- `MANUAL-UPGRADE`, `NOT_RUN` — 실제 0.4.8 설치본 upgrade와 기존 중복 helper 정리.
- `MANUAL-GAME`, `NOT_RUN` — 실제 마비노기 affinity 적용·복원.
- `MANUAL-INSTALL`, `NOT_RUN` — 설치·제거·포터블 UI.

## Failures and retest

- 최초 full Node 실행은 기존 static assertion이 제거된 `previous_status/recent/confirmed` 구현을 요구해 1건 실패했다.
- 새 exact process identity·owner-only drop·status 비삭제·stderr pipe 계약으로 assertion을 갱신했다.
- targeted file과 full Node 213/213 재실행 `PASS`.
- strict clippy 35건은 변경 범위 밖 기존 code style debt이며 이번 checkpoint에서 수정하거나 suppress하지 않았다.

## Coverage assessment

- mandatory automated source·architecture·package scenarios: `COMPLETE_PASS`
- product manual lifecycle: `NOT_RUN`, 실제 설치·게임 동작까지 verified라고 주장하지 않음.
- evidence: `evidence/package.md`
