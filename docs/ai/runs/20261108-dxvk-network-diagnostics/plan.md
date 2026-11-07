# 구현 계획

- record_id: `PLAN-20261108-DXVK-NETWORK-001`
- run_id: `20261108-dxvk-network-diagnostics`
- checkpoint_id: `CP-IMPLEMENT`
- owner: `cursor-agent-92251f36 (DEV)`
- created_at: `2026-11-07T17:13:00Z`
- status: `completed`
- target: `base 4b68b4e7f481761a61121c37bf7ded37519f078e, branch fix/dxvk-network-diagnostics-0411`
- feature_impact: `FEAT-NOGIREM-DXVK, FEAT-NOGIREM-NETWORK-FASTPING — request 기록 범위`
- feature_map: `updated — 완료 시 exact source·검증 freshness 반영`
- architecture_impact: `DXVK transaction adapter와 network compatibility/error projection`
- architecture_contract: `updated — 완료 시 supported path와 enforcement 결과 반영`

## 체크포인트

1. `CP-IMPLEMENT` (`completed`, DEV): NT namespace DXVK rename, filter projection, Kaspersky·권한 안내와 회귀 테스트
2. `CP-QA` (`completed`, DEV-as-QA): targeted tests, backend 전체 test, Node 전체 test, desktop locked check, lint
3. `CP-REVIEW-DOCS` (`completed`, SELF_REVIEW): 실제 diff·주변 transaction·보안 경계 검토, feature map·architecture·handoff 갱신
4. `CP-COMMIT` (`completed`, Coordinator): exact dirty manifest 확인, 소유 파일만 stage·commit, 결과 재확인

## 구현 경로와 stop 조건

- `dxvk.rs`만 DLL transaction writer로 유지한다. `FILE_RENAME_INFO`의 전체 대상 이름을 DOS·extended DOS·UNC 입력에서 명확한 NT namespace 경로로 정규화한다. parent·source·target handle identity 검증은 유지한다.
- 기존 digest, PE, identity, backup, rollback, reparse 검사는 유지한다. broad fallback copy나 보안 예외는 추가하지 않는다.
- Rust와 Electron compatibility projection은 조회·적용 모두 같은 `supported=false` 결과를 반환한다.
- `network-query.ps1`은 `KL_KLIM6`을 계속 차단하되 제품명을 포함한 이유만 제공한다.
- PowerShell access denied는 Rust network adapter의 좁은 오류 formatter에서 분류한다.
- mandatory checks 실패, 새 architecture violation, rollback 회귀 또는 사용자 범위 밖 변경은 checkpoint 완료를 차단한다.

## QA 시나리오

- `DXVK-001/006/007`: 신규·기존 DLL 교체, 한글 parent, canonical extended path, 최종 hash, rollback과 임시 파일 정리
- `DXVK-005`: NotFound·AccessDenied·SharingViolation·InvalidName 안내 분류
- `NET-FP-002`: 이미 설정된 값 보존
- `NET-FP-006`: read-only와 apply 모두 차단 filter를 미지원으로 투영하고 write/restart 0회
- `NET-FP-007`: Kaspersky binding의 구체적 compatibility reason
- `NET-FP-008`: 액세스 거부의 권한·보안 정책 안내

## 검토 요구

- review_mode: `SELF_REVIEW`
- limitation: 같은 agent가 구현·QA·review를 수행하며 독립 reviewer는 참여하지 않는다.
- inspect: path namespace, RootDirectory handle lifetime, backup restore, target identity, Kaspersky allowlist 금지, read-only UI projection, 권한 오류의 과도한 단정 여부
