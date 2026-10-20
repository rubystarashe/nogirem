# Request

- record_id: `REQ-AFFINITY-STDERR-LIFETIME-049`
- run_id: `20261020-affinity-stderr-lifetime-049`
- checkpoint_id: `CP-AFFINITY-STDERR-LIFETIME-049`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-20T07:24:00Z`
- target_reference: `uncommitted target based on 81f0da72451c9feb4016add5da45e41ce749136f`
- original_request: `패키징해`와 후속 `수정해서`
- objective: `0.4.9에서 실시간 부스트 UI와 affinity helper 실제 실행이 분리되는 회귀를 수정하고 signed Windows 설치형·포터블 교체 패키지를 생성한다`
- acceptance_criteria: `helper가 초기 running 상태 이후 stderr 출력 여부와 무관하게 유지되고, 실제 게임 탐지·CPU affinity 적용 경로가 지속되며, signed 0.4.9 패키지가 검증된다`
- scope: `desktop/backend/src/boost.rs, 관련 회귀 테스트, package·run·canonical 기록`
- exclusions: `배포, push, 실제 게임 프로세스 종료, 사용자 VERSION_HISTORY.md 변경, 비관련 dirty 파일`
- risk: `MEDIUM — helper lifecycle과 CPU 최적화 지속성에 직접 영향을 주지만 durable data·외부 API·cross-repository 변경은 없다`
- authorization: `수정과 패키징까지 허용; 배포와 push는 허용되지 않음`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — affinity helper 반복 종료로 CPU 설정이 적용되지 않는 회귀 수정`
- feature_map: `updated — canonical revision 41, feature revision 18`
- architecture_impact: `helper subprocess stderr ownership과 coordinator-worker lifecycle 경계`
- architecture_contract: `updated — revision 45`
- initial_impact_triage: `intake 시 feature map·architecture contract 갱신 필요 여부는 검증 전 unknown이었고 2026-10-20T07:36:00Z에 갱신으로 해소`
