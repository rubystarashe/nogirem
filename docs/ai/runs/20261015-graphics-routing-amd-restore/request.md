# Graphics routing and AMD restore request

- record_id: `REQ-GRAPHICS-RESTORE-001`
- run_id: `RUN-20261015-GRAPHICS-ROUTING-AMD-RESTORE`
- checkpoint_id: `CP-GRAPHICS-RESTORE-001`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-15T13:28:00Z`
- target: `main base 061a23a4c7a6fc977ce8b3ec32e43c691ab8d1c0`
- feature_impact: `FEAT-NOGIREM-GRAPHICS-SETTINGS — 혼합 GPU NVIDIA 선택과 AMD 3D 최적화 복구`
- feature_map: `updated — revision 2`
- architecture_impact: `GPU 선택 우선순위와 Radeon driver API 경계`
- architecture_contract: `updated — revision 29`

## 요청

- immutable_source: 현재 사용자 대화.
- objective: AMD 내장 GPU와 RTX 4090이 함께 있으면 NVIDIA 기능을 표시한다. AMD 단독 시스템에서도 그래픽 최적화 기능을 제공하되 언더볼팅·오버클럭 설정을 해제하지 않는다.
- deliverables: NVIDIA 우선 routing, AMD 최적화 UI·backend·helper 복구, tuning API 금지, 회귀 테스트, canonical docs, rebuilt helper.
- acceptance_criteria: `task.md`의 5개 기준을 따른다.
- scope: `graphics.rs`, Dioxus UI, Radeon helper source·binary, Electron compatibility path, 관련 테스트·문서.
- exclusions: tuning 값 snapshot·복원, AMD driver 수정, package·push·release·deployment.
- risk: `moderate — 명시적 AMD 전역 3D writer를 복구하며 실제 AMD 하드웨어 tuning 보존은 별도 수동 검증이 필요함`
- authorization: source 구현·검증·local commit까지. 외부 게시와 배포는 불허.
- unresolved_question: `AMD 정상 ADLX 초기화가 제보 GPU의 tuning 값을 실제로 보존하는지 hardware owner가 확인해야 함`
