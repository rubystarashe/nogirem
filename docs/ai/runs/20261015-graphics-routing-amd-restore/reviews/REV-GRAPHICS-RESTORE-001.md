# REV-GRAPHICS-RESTORE-001

- run_id: `RUN-20261015-GRAPHICS-ROUTING-AMD-RESTORE`
- checkpoint_id: `CP-GRAPHICS-RESTORE-001`
- review_mode: `INDEPENDENT_REVIEW`
- reviewer: `generalPurpose agent adf951c0-1cb0-4463-8c6b-2751a0e957fe`
- requester: `cursor-agent-92251f36`
- base_target: `061a23a4c7a6fc977ce8b3ec32e43c691ab8d1c0`
- feature_impact: `FEAT-NOGIREM-GRAPHICS-SETTINGS — routing and AMD optimization safety`
- feature_map: `updated — revision 2`
- architecture_impact: `Radeon status/apply boundary and rollback`
- architecture_contract: `updated — revision 29`

## Round 1 request and acknowledgment

- round_id: `REV-GRAPHICS-RESTORE-001-R1`
- requested_at: `2026-10-15T13:35:00Z`
- target: `base + then-current working tree`
- scope: NVIDIA-first mixed routing, AMD WMI-only status, `I3DSettings`-only apply, helper contract, tests and docs.
- acknowledgment: reviewer accessed the target and performed independent diff/source review.

## Round 1 findings

- `ISSUE-GFX-R-001`, HIGH, `must_fix:true`, state `READY_FOR_REREVIEW`: reviewer observed an older `src/radeon.mjs` target where `check()` called `runHelper(false)`.
- `ISSUE-GFX-R-002`, MEDIUM, `must_fix:true`, state `READY_FOR_REREVIEW`: helper could partially apply later settings after one failure and callers could change game VSync before surfacing the helper reason.
- `ISSUE-GFX-R-003`, BLOCKER, `must_fix:true`, state `READY_FOR_REREVIEW`: helper rebuild changed the binary hash during review, invalidating the exact target.
- verdict: `BLOCKED`.
- limitation: R1 did not include the later Electron WMI-only status changes and is stale for that path.

## DEV response

- `ISSUE-GFX-R-001`: agreed with required boundary; latest Rust and Electron status paths both use WMI device detection and never invoke the helper. Added Electron tests for helper call count 0 and missing-name fallback.
- `ISSUE-GFX-R-002`: agreed. Added per-GPU snapshots of Chill, Enhanced Sync, VSync and Anti-Lag before writes. Any apply failure triggers best-effort restoration of every snapshot, followed by final state re-read. Rust and Electron reject helper reason before game VSync and include final goals in the error. Added Rust and Node regression tests.
- `ISSUE-GFX-R-003`: agreed. Rebuilt the helper after the final source change and will submit the resulting SHA-256 and complete manifest for rereview.
- response_target: `base + current working tree after rollback implementation`
- validation: `graphics 5/5; backend 114 PASS·3 ignored + parity 2/2; Radeon Node 12/12; Node 210/210; desktop check; actual RTX 4090 status; helper build PASS`.

## Round 2

- round_id: `REV-GRAPHICS-RESTORE-001-R2`
- target: `exact code hashes ending helper binary f83398c2…c664f`
- `ISSUE-GFX-R-001`: `RESOLVED`.
- `ISSUE-GFX-R-002`: `OPEN — rollback setter 성공을 snapshot 복원 성공으로 오인하고 raw allMet=false/reason=null에서 게임 VSync를 먼저 변경함`.
- `ISSUE-GFX-R-003`: `RESOLVED`.
- new_findings: `none`.
- verdict: `CHANGES_REQUESTED`.

## DEV response round 2

- `ISSUE-GFX-R-002`: rollback과 target write 각각 setter 후 실제 값을 목표값 또는 snapshot과 비교하도록 변경했다. 불일치는 적용 실패 또는 rollback 실패로 기록한다.
- Rust와 Electron은 `reason:null`이어도 `allMet != true`이면 게임 VSync write 전에 최종 goals와 함께 거부한다.
- regression: Rust `unmet_radeon_apply_is_rejected_before_game_write`, Node `Radeon helper 목표 미달은 게임 설정 변경 전에 거부한다`.
- response_validation: `graphics 6/6; backend 115 PASS·3 ignored + parity 2/2; Radeon Node 13/13; Node 211/211; desktop check; actual RTX 4090 status PASS`.

## Round 3

- round_id: `REV-GRAPHICS-RESTORE-001-R3`
- target: `graphics.rs 55110c60…ae6a9; ui.rs fd070ee9…76f1c; main.cpp 2292adc6…248c8; helper e1d3a09a…643e8; src/radeon.mjs e2d60857…9a702; test/radeon.test.mjs d67bcbf3…b6dd`
- status: `COMPLETE`
- `ISSUE-GFX-R-001`: `RESOLVED`.
- `ISSUE-GFX-R-002`: `RESOLVED`.
- `ISSUE-GFX-R-003`: `RESOLVED`.
- new_findings: `none`.
- rerun: `Radeon Node 13/13 PASS; Rust graphics 6/6 PASS; git diff --check PASS`.
- limitations: `AMD 단독 실장비 적용과 언더볼팅 유지 확인은 NOT_RUN이며 비차단 QA 한계로 유지`.
- verdict: `APPROVED`.
- approval_freshness: `APPROVED for the six exact code hashes recorded above; subsequent run-document-only changes do not affect reviewed behavior`.
