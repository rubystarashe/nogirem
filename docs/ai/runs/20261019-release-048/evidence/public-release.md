# 0.4.8 public release evidence

- evidence_id: `EVID-RELEASE-048-001`
- producer: `cursor-agent-92251f36`
- created_at: `2026-10-19T09:23:00Z`
- run_id: `20261019-release-048`
- checkpoint_id: `CP-RELEASE-048`
- source_target: `af36bc1e1e905864914c8fea4bfd2e7a17cd25fa`
- coordination_push_target: `16feba774a41aaa1f09df0e32a6b2ca3ba2576c6`
- execution_target: `GitHub rubystarashe/nogirem v0.4.8`
- release_url: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.8`
- candidate: `release/ready-v0.4.8-2026-10-19T09-20-30-587Z`
- public_download: `release/public-v0.4.8-2026-10-19T09-23-00Z`
- feature_impact: `FEAT-NOGIREM-FRAME-BOOST — 균형 기본 CPU 모드와 CPU 개선을 0.4.8로 공개`
- feature_map: `updated — canonical revision 35, feature revision 14에 공개 배포 evidence 연결`
- architecture_impact: `기존 signed package, Ed25519 update, Electron migration bridge와 GitHub Release 경계 실행`
- architecture_contract: `no_change — revision 41의 배포 경계를 exact public target에 재평가`

## 공개 상태

- `PASS` — origin/main `16feba774a41aaa1f09df0e32a6b2ca3ba2576c6`
- `PASS` — annotated `v0.4.8` tag object `9909ec0d6d8be82356045277f661a34d3715daff`, target `af36bc1e1e905864914c8fea4bfd2e7a17cd25fa`
- `PASS` — public, non-draft, non-prerelease, Latest Release
- `PASS` — canonical asset 9개 공개
- `PASS` — 공개 재다운로드 9/9가 candidate SHA-256와 일치
- `PASS` — installer·portable public aliases가 canonical EXE와 동일
- `PASS` — 공개 update·portable manifest Ed25519, version `0.4.8`, size와 SHA-256
- `PASS` — Electron `latest.yml` version `0.3.19`, migration EXE SHA-512와 blockmap
- `PASS` — turbo helper asset 부재

## 핵심 자산

- installer: `nogirem-dioxus-setup-0.4.8.exe`와 `nogirem-setup-0.4.8.exe`, `9,823,145 bytes`, SHA-256 `3c697b541da666ed4c1127bbc2cfc73b0eb58a0d89dc8f84369fbe9fc6f0e0fe`
- portable: `nogirem-dioxus-portable-0.4.8.exe`와 `nogirem-portable-0.4.8.exe`, `7,836,262 bytes`, SHA-256 `cf3d565bcda0888290e1e21934d14773313418623d2b521f72e0bfb67887415e`
- update_manifest: `update.json`, SHA-256 `c199994eecf3156a2e5a66d1af359e5eb0955f281bb094817b1e6568c48508bc`
- portable_manifest: `portable-update.json`, SHA-256 `c3fde98e028b3f5077525321fc0a711d7126851472035da926fb06756f6c81f7`
- electron_bridge: `nogirem-legacy-electron-migration-0.3.19.exe`, SHA-256 `63da5da24ed1aa89fb825ed8a0634d97a56114bd2eab9a4c0620fd48834ea366`

## 한계

- 실제 설치·제거·포터블 실행·자동 업데이트·게임은 `NOT_RUN`.
- publication review는 `SELF_REVIEW`; 독립 reviewer가 참여하지 않았다.
- private signing key와 credential은 기록하지 않았다.
