# Source manifest

- evidence_id: `EVID-CPU-TOPOLOGY-SOURCE-001`
- run_id: `20261019-cpu-topology-priority`
- checkpoint_id: `CP-CPU-TOPOLOGY-PRIORITY-001`
- producer: `cursor-agent-92251f36`
- created_at: `2026-10-19T05:35:00Z`
- repository: `C:\Users\User\Documents\GitHub\nogirem`
- branch: `main`
- base_commit: `1502358cc77df6a35e3515cf5ec98ddd2f0737ab`
- index_state: `no pre-existing staged files`
- worktree_state: `owned modified files below; run records and .handoff tracked separately`
- out_of_scope_dirty: `NOTICE.md modified; data/ untracked; both preserved and excluded`
- patch_sha256: `f0f3b3d3c3eb1a869ea39f19cc59b569c2cff913865fd98604b128dfe0c30c18`
- patch_procedure: `git diff over the listed files, joined with LF without a final extra line, UTF-8 encoded, SHA-256`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — topology, affinity, priority and recovery`
- feature_map: `updated — feature-map revision 31 and feature revision 2`
- architecture_impact: `topology authority, same-handle process adapter, durable ledger and IPC`
- architecture_contract: `updated — revision 35`

## SHA-256

```text
34592603a658c4d75683c7b1afd4f211a40bffde1a0f89ea7171a524c0103f17  desktop/backend/src/affinity.rs
94956aa8302419500c7e94aefdd8c65eb4084d403e7c49c72206ad7efc815939  desktop/backend/src/affinity_worker.rs
bb713eeaf3326f97b9482a701349b5fcfd9e619cbe7172e3b50bae0618b15154  desktop/backend/src/bin/nogirem-cli.rs
eabaee53a04d744fb1fe0806bd9b8f00d23c6c5c89cf398e66da2a3afedff5db  desktop/backend/src/boost.rs
a4b2ce83cc2137516aa75be014dfa3da53d5ffb0ef732ddf8f502be5523cb887  desktop/backend/src/process.rs
45b3326147ec47c4d1879f10c95a5eb959bd92aa410ede3d3958f1a7990b9ed3  desktop/backend/src/service-contract.json
b427e6955e09aede3c317fb0b13c3240be4f398e2a382340715f0769bec4d00d  desktop/backend/src/service.rs
e34e18ab278f9c5998320e7c50da5d24cd5351a432c32300f938fa337cfdc76a  desktop/backend/src/service_fixture.rs
423e7127fe009e530b7ade3616f25b56a1f26c68294314aedb0afee8504d22de  desktop/backend/src/settings.rs
e4d6f3b35aa2ab8e2a7ca0acaff24ae440b2fc9c0d2d3ea7426473e3731152eb  desktop/backend/src/topology.rs
86ace87f6858239fe31961734e0498a1ca2e951f69f4920fdf8d2881ca2bcf53  desktop/backend/src/workers.rs
800435154f877ff6254685b04c8d09c3cf7b8fff4eefca23c7d382f3e2ac8126  desktop/backend/tests/legacy-parity.rs
0b6176cf2c6eb7dadce9c65290c7cc3f4c4892b559af536cdf21267a1c059995  desktop/src/ui.rs
6bbac27fc6be8df87125fabee725157741f6eca7c8ba81e71e0b9e30e71ffab6  docs/ai/wiki/agent-friendly-architecture.md
79cfd03e8138777cadd035309eec8c4654e72e3368c6e639eebd3daeaf56d326  docs/ai/wiki/feature-map.md
```

File hashes use PowerShell `Get-FileHash -Algorithm SHA256` against the working-tree bytes. No symlinks, submodules, renames, deletions or binary files are included.
