# Request

- record_id: `REQ-STARTUP-STATUS-049`
- run_id: `20261020-startup-status-fallback-049`
- checkpoint_id: `CP-STARTUP-STATUS-049`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-20T10:05:00Z`
- original_request: `해당 조회를 재시도하고, 실패해도 앱은 정상 시작하되 자동 시작 설정만 확인 불가로 처리하고 패키징·대치배포`
- objective: `Windows 예약 작업의 일시적 E_FAIL을 자동 시작 설정에만 격리하고 public v0.4.9를 교체한다`
- acceptance_criteria: `3회 조회 재시도, 최종 실패 시 앱 정상 시작, 자동 시작 버튼 확인 불가·비활성, 명시적 설정 변경 실패는 오류 유지, signed package와 public asset 검증`
- scope: `Rust startup adapter, Dioxus 상태·표시, 회귀 테스트, package, origin/main과 GitHub v0.4.9 replacement`
- exclusions: `예약 작업 삭제·재등록 정책 변경, CPU boost 동작 변경, 실제 Windows 장애 강제 주입`
- risk: `HIGH — 앱 시작 failure containment은 국소적이나 같은 버전 public release 자산을 대치함`
- authorization: `사용자가 구현·패키징·대치배포를 명시적으로 승인`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — Windows 시작 트레이 설정 조회 실패 격리와 public 0.4.9 replacement`
- feature_map: `updated — source·public verification freshness 갱신 필요`
- architecture_impact: `PowerShell 예약 작업 read adapter 실패가 app launch boundary를 중단하지 않도록 제한`
- architecture_contract: `updated — failure containment와 public target freshness 갱신 필요`
