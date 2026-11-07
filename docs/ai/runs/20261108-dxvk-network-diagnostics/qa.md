# QA 기록

- record_id: `QA-20261108-DXVK-NETWORK-001`
- run_id: `20261108-dxvk-network-diagnostics`
- checkpoint_id: `CP-QA`
- owner: `cursor-agent-92251f36 (DEV-as-QA)`
- updated_at: `2026-11-07T17:28:00Z`
- qa_state: `COMPLETE_PASS`
- source_target: `base 4b68b4e7f481761a61121c37bf7ded37519f078e; source hashes in evidence/source-manifest.md`
- execution_target: `Windows 10.0.26200 x64, Rust locked test/check, Node 22.20.0 with npm ci dependencies`
- feature_impact: `FEAT-NOGIREM-DXVK, FEAT-NOGIREM-NETWORK-FASTPING`
- feature_map: `updated — 결과에 따라 canonical evidence 반영 예정`
- architecture_impact: `DXVK transaction·network compatibility와 PowerShell error adapter`
- architecture_contract: `updated — 구조 검사 결과 반영 예정`

## 필수 시나리오

- `DXVK-001-NT-NAMESPACE`: `PASS` — 한글 parent의 기존 target에서 오류 123을 회귀로 재현한 뒤 DOS·extended DOS·UNC를 `\??\` namespace로 변환해 신규·교체 모두 통과
- `DXVK-005-ERRORS`: `PASS` — 오류 2·5·32·123 안내 분류
- `DXVK-007-ROLLBACK`: `PASS` — 후속 검증 실패 시 기존 DLL 복구, 원본 없는 target 제거, backup 보존과 temporary cleanup
- `NET-FP-006-BLOCKED-READ`: `PASS` — read-only와 apply 모두 `supported=false`, `compatibilityBlocked=true`, write/restart 0회; manager `optimized=true`
- `NET-FP-007-KASPERSKY`: `PASS` — `KL_KLIM6` 제품명 안내와 compatible allowlist 부재
- `NET-FP-008-ACCESS-DENIED`: `PASS` — 오류 5에서 관리자 권한·보안 프로그램·시스템 정책 지침과 원문 보존
- `REGRESSION-BACKEND`: `PASS` — `cargo test --manifest-path desktop/backend/Cargo.toml --locked`, 137 PASS·3 ignored, parity 2/2 PASS
- `REGRESSION-NODE`: `PASS` — `npm ci` 후 `npm test`, 216/216 PASS
- `DESKTOP-CHECK`: `PASS` — `cargo check --manifest-path desktop/Cargo.toml --locked`
- `IDE-LINT`: `PASS` — 변경 source/test 6개 diagnostics 0
- `DIFF-CHECK`: `PASS` — `git diff --check`
- `RUSTFMT-REPOSITORY`: `FAIL (pre-existing, non-blocking)` — `cargo fmt -- --check`는 변경 worktree와 clean `4b68b4e` worktree 모두 repository 기존 대규모 formatting drift로 실패했다. 이번 diff에 whitespace 오류는 없다.
- `EXPLORATORY-NODE-DISCOVERY`: `FAIL (superseded)` — 잘못된 광범위 명령 `node --test`가 scripts smoke·native integration까지 발견해 미설치 dependency와 unrelated parallax threshold로 189 PASS·7 FAIL이었다. 정본 `npm test`를 dependency 설치 후 실행해 216/216 PASS로 대체했으며 실패 항목은 이번 source 범위 검증 결과로 사용하지 않는다.

## 실제 결과와 제한

- expected: 기존 writer·reparse·digest·backup 경계를 유지하면서 오류 123, filter false alarm, 오류 5 안내를 고친다.
- actual: 자동 회귀가 expected contract를 충족했고 새 write path·allowlist·보안 설정 변경은 없다.
- manually_exercised: `NOT_RUN`
- blocked_or_not_run: 제보 PC의 DXVK v3.1.1 설치, Kaspersky 장착 NIC UI, ACL/보안 정책 차단은 실행 환경이 없어 `NOT_RUN`
- cleanup: test temporary paths는 각 회귀가 정리했고 npm dependency는 ignored `node_modules`에만 설치됨
