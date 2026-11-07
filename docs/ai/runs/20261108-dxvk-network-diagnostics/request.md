# 요청 기록

- record_id: `REQ-20261108-DXVK-NETWORK-001`
- run_id: `20261108-dxvk-network-diagnostics`
- checkpoint_id: `CP-IMPLEMENT`
- owner: `cursor-agent-92251f36 (Coordinator, DEV, QA, Reviewer, Feature owner, Architecture owner, Documentation maintainer)`
- created_at: `2026-11-07T17:12:00Z`
- target: `nogirem fix/dxvk-network-diagnostics-0411, base 4b68b4e7f481761a61121c37bf7ded37519f078e`
- source_reports: `Hojoring 36852419-7c90-4477-b3a3-12551d368c1c, 지원김 ccb6626e-3d05-4367-aad3-5ae865b88a2b, zero 34d51a15-5c7e-4278-ab48-073eee9c00a9`
- feature_impact: `FEAT-NOGIREM-DXVK — v3.1.1 저장 교체의 Windows 경로 호환성·오류 안내; FEAT-NOGIREM-NETWORK-FASTPING — 차단 필터 상태·Kaspersky 안내·권한 오류`
- feature_map: `updated — 구현과 검증 결과를 canonical 항목에 반영 예정`
- architecture_impact: `DXVK same-directory handle transaction의 rename 대상 표현; network compatibility projection과 PowerShell 오류 경계`
- architecture_contract: `updated — canonical transaction·filter·권한 안내 경계를 실제 구현과 맞춤 예정`

## 원 요청과 목표

사용자는 세 개의 0.4.10 진단을 제공한 뒤 확인된 문제를 모두 개선하도록 요청했다.

1. DXVK v3.1.1 저장 단계의 `os error 2`와 `os error 123`을 백신으로 단정하지 않고, 절대 경로 이름 변경 호환 문제를 제거한다.
2. Kaspersky `KL_KLIM6`가 연결된 새 네트워크 어댑터를 조회만 해도 미적용 상태로 오인하는 표시를 수정하고 차단 제품명을 안내한다.
3. 패스트핑 PowerShell이 `액세스 거부`로 실패하면 관리자 권한 또는 보안 정책 차단임을 사용자가 구분할 수 있게 한다.

## 범위와 제외

- 범위: `desktop/backend/src/dxvk.rs`, Rust·Electron 네트워크 projection, 네트워크 PowerShell binding 분류, 관련 테스트와 canonical 문서
- 제외: Kaspersky filter 자동 해제·허용, Windows 보안 예외 변경, release 버전 변경, 패키징, push, tag, GitHub Release, 배포
- acceptance:
  - DXVK 임시 파일은 DOS·extended DOS·UNC 경로를 명확한 NT namespace 이름으로 변환해 handle 기반으로 최종 교체된다.
  - 한글 경로 교체와 기존 파일 rollback 회귀가 자동 검사로 통과한다.
  - read-only 상태에서도 비호환 filter는 `supported=false`, `compatibilityBlocked=true`로 반환된다.
  - `KL_KLIM6`은 Kaspersky 네트워크 필터로 명시된다.
  - 액세스 거부 오류는 원문을 보존하면서 권한·보안 정책 확인 지침을 포함한다.

## 위험과 권한

- risk: `MEDIUM` — durable game/app DLL transaction과 관리자 registry 경계를 건드리지만 writer·대상 파일·registry 경로·보안 차단 정책은 늘리지 않는다.
- authorization: source·tests·문서 수정과 로컬 commit까지 허용됨
- prohibited: 외부 게시·배포·보안 제품 설정 변경
- unresolved: 제보 PC 실기기 재현은 현재 환경에서 불가능하므로 자동 회귀와 API 계약까지만 검증한다.
