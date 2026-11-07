# Request

- record_id: `REQ-PACKAGE-0411`
- run_id: `20261108-package-0411`
- checkpoint_id: `CP-VERSION-0411`
- owner: `cursor-agent-92251f36 (Coordinator)`
- created_at: `2026-11-07T17:33:00Z`
- original_request: `0.4.11로 패키징해`
- objective: `진단 세 건 개선을 0.4.11로 기록하고 clean committed source에서 signed 설치형·포터블을 생성·검증`
- source_base: `6aaa1c6e5beb5bfed1fda2e0466b165eadfc4d9f`
- user_history_input: `기본 worktree VERSION_HISTORY.md 변경은 source branch에 이미 동일하게 포함됨; SHA 비교 후 새 0.4.11 section만 추가`
- scope: `VERSION_HISTORY, package/Cargo/lock versions, automated checks, local commits, signed package와 manifests, run·canonical docs`
- exclusions: `push, tag, GitHub Release, public asset replacement, deployment, reporter-PC installation`
- acceptance: `0.4.11 version identity 일치, mandatory checks PASS, signed installer·portable·manifests와 payload contract PASS, exact hashes 기록`
- authorization: `source 변경, local commit, signed local package 생성`
- risk: `HIGH — 실행 가능한 signed installer·portable과 update manifests를 생성하므로 exact committed source와 package security contract 검증 필요`
- feature_impact: `FEAT-NOGIREM-DXVK, FEAT-NOGIREM-NETWORK-FASTPING — 0.4.11 package; FEAT-NOGIREM-UPDATE-LIFECYCLE — signed local artifacts`
- feature_map: `updated — package exact target과 evidence 반영 예정`
- architecture_impact: `version identity, package payload, Ed25519 manifest와 update routing`
- architecture_contract: `updated — package enforcement evidence 반영 예정`
