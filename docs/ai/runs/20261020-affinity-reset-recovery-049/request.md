# Request

- record_id: `REQ-AFFINITY-RESET-RECOVERY-049`
- run_id: `20261020-affinity-reset-recovery-049`
- checkpoint_id: `CP-AFFINITY-RESET-RECOVERY-049`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-20T08:03:00Z`
- target_reference: `uncommitted target based on 4babd317bfef7e325bcf83aca6dd5319888acc8a`
- original_request: `수정해임마`, 후속 `수정하고 재패키징해`
- objective: `부스트 중단 시 이전 crash loop가 남긴 ledger 밖 CPU affinity까지 복구하고 signed 0.4.9 패키지를 재생성한다`
- acceptance_criteria: `현재 ledger, durable applied marker, ledger 밖의 exact app-generated mask가 reset에서 복구되고 외부 변경·다른 mask는 보존된다`
- scope: `affinity manager·worker reset recovery, tests, signed Windows package, canonical/run records`
- exclusions: `push, 배포, 사용자가 직접 지정한 app mask와 구분 불가능한 값에 대한 무조건적 전역 변경`
- risk: `HIGH — 실행 중인 다른 프로세스의 CPU affinity를 복구하는 mutation 경계이며 잘못된 대상은 사용자 설정을 훼손할 수 있음`
- authorization: `수정과 재패키징 허용; push·배포 미허용`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — boost 중단 시 stale affinity 복구`
- feature_map: `updated — canonical 42, feature 19`
- architecture_impact: `durable affinity marker, original-state ledger와 legacy recovery writer 경계`
- architecture_contract: `updated — revision 46`
