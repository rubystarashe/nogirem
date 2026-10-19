# Final report — CPU topology and priority

- record_id: `FINAL-CPU-TOPOLOGY-PRIORITY-001`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-TOPOLOGY-PRIORITY-001`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-19T05:35:00Z`
- disposition: `COMPLETED`
- source_target: `base 1502358cc77df6a35e3515cf5ec98ddd2f0737ab + EVID-CPU-TOPOLOGY-SOURCE-001`
- patch_sha256: `f0f3b3d3c3eb1a869ea39f19cc59b569c2cff913865fd98604b128dfe0c30c18`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — topology, non-overlapping affinity, advanced binding/priority and recovery`
- feature_map: `updated — canonical revision 31, feature revision 2`
- architecture_impact: `topology authority, same-handle process adapter, durable original-state ledger and narrow settings IPC`
- architecture_contract: `updated — revision 35`

## Delivered

- Combined CPU Set efficiency/scheduling data with GLPIEx physical-core, SMT, L3 and processor-group relationships.
- Allocated only the requested P-cores to the game mask, preferring one L3 domain, and assigned remaining P-cores plus E-cores to a disjoint background mask.
- Disabled CPU splitting with a visible reason on four-or-fewer physical cores, multiple processor groups or malformed/unrepresentable topology.
- Made game hard affinity opt-in and background `BELOW_NORMAL_PRIORITY_CLASS` a separate default-false option limited to the general background role.
- Added same-handle process identity, write-before-mutation original-state ledger, current-equals-applied restore guard, startup recovery and failed-entry retention.
- Added normalized persistence, live helper control, service channels, Dioxus controls and updated canonical records.

## Verification and review

- Automated: backend 124 passed·3 ignored plus integration parity 2/2; desktop 8/8; Node 213/213; desktop locked check passed.
- Manually exercised by test process: live Windows topology query and child-process priority apply/restore roundtrip.
- `NOT_RUN`: actual Mabinogi frame/input behavior and packaged helper forced-exit recovery UI.
- Strict Clippy: command executed but nonzero on repository baseline warnings introduced by Rust 1.98; no suppression or baseline expansion. The introduced constructor-argument warning was resolved.
- review_mode: `SELF_REVIEW`; verdict `APPROVED`; `ISSUE-CPU-001`~`003` resolved; no independent reviewer.

## Gates and remaining risk

- Feature gate: `PASS` for implemented automated scope; actual game performance remains an explicit evidence gap.
- Architecture gate: `PASS`; no truncated multi-group mask, duplicate writer or broad exception.
- Review/QA gate: `PASS` with disclosed SELF_REVIEW and Clippy baseline limitation.
- Target/authorization gate: `PASS`; `NOTICE.md` and `data/` preserved and excluded.
- Commit: one local checkpoint commit contains the owned target and this report; its SHA is reported outside the containing commit to avoid a self-reference cycle.
- Push/deploy/release: not performed and not authorized.

## Final current disposition — CP-CPU-CUSTOMIZED-005

- disposition: `COMPLETED`
- target: `base 60588e80b2002df7e160a4fa469e2301090e2dd9 + owned customized-dropdown target committed with this report; resulting SHA is reported externally`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — standard/custom CPU preset dropdown and advanced safe toggle`
- feature_map: `updated — revision 6`
- architecture_impact: `effective preset projection and preserving advanced IPC; topology/ledger boundaries unchanged`
- architecture_contract: `updated — revision 39`
- delivered: main dropdown의 `게임 집중 / 균형 / 안전 모드`, nonstandard mode/count일 때 동적 `사용자화` option, advanced count와 safe toggle 상호 보존.
- automated-test-passing: backend tests, desktop 8/8, Node 213/213, desktop locked check, corrected main smoke and edited-file lints.
- manually exercised: smoke DOM event로 standard 세 preset 전환과 advanced 5P+safe 후 `사용자화` 표시를 실행했다.
- actual Mabinogi performance: `NOT_RUN`.
- review: `REV-CPU-CUSTOMIZED-005-R2 APPROVED`, `SELF_REVIEW`; no independent reviewer and no unresolved mandatory finding.
- gates: feature `PASS` for implemented UI/settings scope; architecture `PASS`; review/QA `PASS`; target/authorization `PASS`.
- commit/push/deploy: follow-up local checkpoint contains this report and its resulting SHA is reported externally; push/deploy/release not performed or authorized.

## Final current disposition — CP-CPU-UI-LAYOUT-006

- disposition: `COMPLETED`
- target: `base 60e83bbb97f86e96d9051f230c2afbaf54757b94 + owned UI-layout target committed with this report; resulting SHA is reported externally`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — independent advanced CPU section and preset pending feedback`
- feature_map: `updated — revision 7`
- architecture_impact: `none — service/data/topology boundaries unchanged`
- architecture_contract: `no_change — revision 39 remains accurate`
- delivered: independent advanced CPU section; transparent borderless larger preset dropdown with increased top spacing; pending `설정 변경중`.
- automated-test-passing: desktop locked check, desktop 8/8, main smoke and edited-file lints.
- manually exercised: smoke DOM state and screenshot visual review.
- review: `REV-CPU-UI-LAYOUT-006-R1 APPROVED`, `SELF_REVIEW`; no independent reviewer and no unresolved mandatory finding.
- actual game workload: `NOT_RUN`.
- commit/push/deploy: local checkpoint contains this report and its resulting SHA is reported externally; push/deploy/release not performed or authorized.

## Final current disposition — CP-CPU-DROPDOWN-009

- disposition: `COMPLETED`
- target: `base 4df74e239ffaad517911a46f62682958337fdeec + owned label/inset target committed with this report; resulting SHA is reported externally`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — boost mode label and compact arrow placement`
- feature_map: `updated — revision 10`
- architecture_impact: `none — visual text and spacing only`
- architecture_contract: `no_change — revision 39 remains accurate`
- delivered: `부스트 모드 선택` label; 110px field, 15px right padding and 6px chevron inset.
- automated-test-passing: desktop locked check, clean-process main smoke and edited-file lints.
- evidence limitation: first smoke hit an unrelated transient WebView2 tab error; exact rerun passed.
- review: `REV-CPU-DROPDOWN-009-R1 APPROVED`, `SELF_REVIEW`; no independent reviewer.
- actual game workload: `NOT_RUN`.
- commit/push/deploy: local checkpoint contains this report and its resulting SHA is reported externally; push/deploy/release not performed or authorized.

## Final current disposition — CP-CPU-DROPDOWN-008

- disposition: `COMPLETED`
- target: `base 593bd05a5829b07a951583429308fd35930a450c + owned spacing/copy target committed with this report; resulting SHA is reported externally`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — boost mode chevron/spacing and safe-mode terminology`
- feature_map: `updated — revision 9`
- architecture_impact: `none — visual and user-facing copy only`
- architecture_contract: `no_change — revision 39 remains accurate`
- delivered: font-independent CSS chevron, larger top margin, tighter ruby/select spacing and `게임 CPU 선호도` wording.
- automated-test-passing: desktop locked check, main smoke and edited-file lints.
- manually exercised: computed spacing/chevron/copy assertions and screenshot review.
- review: `REV-CPU-DROPDOWN-008-R1 APPROVED`, `SELF_REVIEW`; no independent reviewer and no unresolved mandatory finding.
- actual game workload: `NOT_RUN`.
- commit/push/deploy: local checkpoint contains this report and its resulting SHA is reported externally; push/deploy/release not performed or authorized.

## Final current disposition — CP-CPU-DROPDOWN-007

- disposition: `COMPLETED`
- target: `base f8c4411 + owned dropdown/action target committed with this report; resulting SHA is reported externally`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — boost mode presentation and top-level advanced CPU options`
- feature_map: `updated — revision 8`
- architecture_impact: `none — service/data/topology boundaries unchanged`
- architecture_contract: `no_change — revision 39 remains accurate`
- delivered: boxless transparent dropdown with ruby label/custom arrow/color transition; normalized mode names; core count/safe/priority top-level actions before CPU reorder.
- automated-test-passing: desktop locked check, desktop 8/8, main smoke and edited-file lints.
- manually exercised: computed style, DOM order/actions, preset roundtrip and screenshot review.
- review: `REV-CPU-DROPDOWN-007-R1 APPROVED`, `SELF_REVIEW`; no independent reviewer and no unresolved mandatory finding.
- actual game workload: `NOT_RUN`.
- commit/push/deploy: local checkpoint contains this report and its resulting SHA is reported externally; push/deploy/release not performed or authorized.
