# Request

- record_id: `REQ-WIN10-CACHE-TOPOLOGY`
- run_id: `20261021-win10-cache-topology`
- checkpoint_id: `CP-WIN10-CACHE-TOPOLOGY`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-21T14:04:00Z`
- request: `Windows 10에서 GetLogicalProcessorInformationEx group mask array is malformed로 CPU 분할이 비활성화되는 문제 개선`
- objective: `legacy CACHE_RELATIONSHIP GroupCount=0을 단일 GroupMask 형식으로 안전하게 해석`
- acceptance: `Windows 10 cache record parse 성공, GroupCount>0 배열 검증 유지, malformed record 거부 유지, 관련 자동 검사 통과`
- scope: `desktop/backend/src/topology.rs, 회귀 테스트, VERSION_HISTORY와 canonical records`
- exclusions: `package, push, tag, GitHub Release, 실제 타 사용자 PC 실행`
- risk: `MEDIUM — CPU affinity 계산 입력을 복원하지만 변경은 read-only Win32 topology parser 한 분기에 제한되고 durable data·외부 API는 변경하지 않음`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — Windows 10 CPU 분할 지원 복구`
- feature_map: `updated — source target 완료 시 freshness 갱신`
- architecture_impact: `Windows GLPIEx cache topology adapter의 legacy/current 구조 호환`
- architecture_contract: `updated — 호환 파싱 규칙 기록 필요`
