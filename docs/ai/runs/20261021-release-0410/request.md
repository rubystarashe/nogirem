# Request

- record_id: `REQ-RELEASE-0410`
- run_id: `20261021-release-0410`
- checkpoint_id: `CP-RELEASE-0410`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-21T14:32:00Z`
- original_request: `0.4.10 수정사항으로 기록하고 패키징하고 배포`
- objective: `Windows 10 CPU topology 호환 수정을 0.4.10으로 기록하고 signed 설치형·포터블과 update assets를 public Latest로 배포`
- acceptance: `버전 정합성, 전체 자동 검사, clean signed package, origin/main과 annotated v0.4.10, GitHub public assets와 서명·hash·routing 검증`
- scope: `0.4.10 version/history, existing Windows 10 parser fix, release records, package, push, tag, GitHub Release`
- exclusions: `실제 제보 PC 설치·게임 프레임 QA, GPU TDR·MEMORY_CORRUPTION 시스템 장애 수정`
- authorization: `local commit, origin/main push, tag, GitHub Release 배포 승인; production 사용자 장치 설치는 미승인`
- risk: `HIGH — public update channel과 signed downloadable artifacts를 변경하므로 exact source·asset·서명·공개 재다운로드 검증 필요`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST, FEAT-NOGIREM-UPDATE-LIFECYCLE — Windows 10 CPU 분할 복구의 public 0.4.10`
- feature_map: `updated — exact public target과 evidence 반영 필요`
- architecture_impact: `GLPIEx legacy cache parser와 기존 signed package/update routing`
- architecture_contract: `updated — exact public target과 enforcement evidence 반영 필요`
