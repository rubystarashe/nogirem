# Source manifest

- evidence_id: `EVD-20261108-SOURCE-001`
- producer: `cursor-agent-92251f36`
- created_at: `2026-11-07T17:25:00Z`
- run_id: `20261108-dxvk-network-diagnostics`
- checkpoint_id: `CP-QA`
- repository: `nogirem`
- branch: `fix/dxvk-network-diagnostics-0411`
- base_commit: `4b68b4e7f481761a61121c37bf7ded37519f078e`
- index_state: `no pre-existing staged changes`
- working_tree_state: `listed source files modified; run·canonical documentation added or modified`
- rename_delete_binary_submodule: `none`
- out_of_scope_dirty_changes: `none in isolated worktree`
- creation_procedure: `PowerShell Get-FileHash -Algorithm SHA256 on exact source/test paths; git status and diff reviewed separately`
- feature_impact: `FEAT-NOGIREM-DXVK, FEAT-NOGIREM-NETWORK-FASTPING`
- feature_map: `updated — revision 47`
- architecture_impact: `DXVK NT rename namespace와 network filter/error adapter`
- architecture_contract: `updated — revision 51`

## Source and test hashes

- `1b649d1e43385725c05f54ed8ca74b4be7abe9f8346e81dc741325443589d57c  desktop/backend/src/dxvk.rs`
- `a72567c7f26de6dca41aa2abc562083a424a1f659a5c6894c0b1ee438366f448  desktop/backend/src/network.rs`
- `9affb3abf0a1c9d0b3db5ccc91ca730ed30d9ebb97ea4bf7223bb0449dc5ad6f  desktop/backend/src/network_manager.rs`
- `2634a4f90c5cad6ddbad3055da9314429eec848278ed69c985b11a225fa4b4ab  desktop/backend/src/powershell/network-query.ps1`
- `b9d959df39a828922691eac85bb03348c33075f058e297db9a2d70400bb76167  src/network.mjs`
- `92634168491f71f709a13abfacf22719ee6fa3743ccdf5c0d28276ba9fc47586  test/network.test.mjs`

`git diff`와 staged patch SHA-256은 최종 review·commit 단계에서 기록한다. 이 manifest의 hash는 commit 후 별도 계산해 final report에 기록한다.
