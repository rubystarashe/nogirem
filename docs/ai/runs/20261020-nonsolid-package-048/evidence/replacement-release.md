# 0.4.8 non-solid package 대치 배포 evidence

- evidence_id: `EVID-NONSOLID-048-REPLACEMENT-001`
- producer: `cursor-agent-92251f36`
- created_at: `2026-10-19T18:54:00Z`
- run_id: `20261020-nonsolid-package-048`
- checkpoint_id: `CP-NONSOLID-REPLACEMENT-048`
- source_target: `a2db18fb08eecf201de4d77759c814bb74c3eaa8`
- candidate_target: `release/ready-v0.4.8-2026-10-19T18-50-42-158Z`
- public_download_target: `release/public-v0.4.8-nonsolid-2026-10-19T18-53-00Z`
- release: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.8`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — non-solid·single-helper 0.4.8 package와 external WebView2 prerequisite 공개 대치`
- feature_map: `updated — canonical revision 39, feature revision 12`
- architecture_impact: `documented revision 43 package bootstrap·compression·helper 경계를 public target에 실행`
- architecture_contract: `no_change — revision 43의 현재 source/package 경계와 일치`

## 공개 자산

- installer canonical·alias: `8,014,317 bytes`, SHA-256 `acb7e89645aa2add439d9e14f59e711cd367e03753314020be17023705bd7745`
- portable canonical·alias: `7,789,628 bytes`, SHA-256 `b46afe5c9aac5e93c9c02e851707cd670ad398720942784cd470c908f98b3df2`
- `update.json`: `422 bytes`, SHA-256 `6f5a2e4ff8a731afa5f701f0cc31838af9a99be5f60aa94865e409026109f808`
- `portable-update.json`: `425 bytes`, SHA-256 `aa2ac7ba06d2893cd99c8af36ef2e271e58310ea4c2421865176fd90fe49db1a`
- Electron `latest.yml`: asset ID `655808752`, SHA-256 `419a0e6c1bb6a9785df5d089ea2ba4188090dff807f940028a840e093a9bcc14`
- Electron migration exe: asset ID `655808750`, SHA-256 `63da5da24ed1aa89fb825ed8a0634d97a56114bd2eab9a4c0620fd48834ea366`
- Electron blockmap: asset ID `655808755`, SHA-256 `4cefd458f9ed2878568e897bc5765f0d6991696436af82b96dccb05b85212d96`

## 공개 검증

- `PASS` — 공개 9/9 자산을 재다운로드하고 candidate SHA-256과 비교.
- `PASS` — canonical·public alias installer와 portable이 각각 byte-identical.
- `PASS` — installer·portable manifest의 Ed25519 signature, version `0.4.8`, payload size와 SHA-256.
- `PASS` — 실행 파일 4개 선행, signed manifest 2개 후행의 feed-safe 교체 순서.
- `PASS` — Electron bridge 3개 자산의 asset ID와 content identity 보존.
- `PASS` — release는 public Latest이며 draft·prerelease가 아님.
- `PASS` — release note에 package 분석성·용량·WebView2 공식 다운로드 안내 반영.
- `PASS` — annotated tag object `9909ec0d6d8be82356045277f661a34d3715daff`와 target `af36bc1e1e905864914c8fea4bfd2e7a17cd25fa` 불변.
- `NOT_RUN` — 실제 설치·제거·포터블 실행·same-version 자동 재수신.
- review_mode: `SELF_REVIEW`
- review_verdict: `APPROVED`
- independent_reviewer: `none`

## 한계

- 동일 `0.4.8`을 이미 설치한 클라이언트는 version 비교상 대치 자산을 자동 재수신하지 않을 수 있어 필요 시 수동 재설치가 필요하다.
- VirusTotal 공개 결과는 사용자가 제출한 installer candidate에 한정되며 본 배포 검증은 VirusTotal 재제출을 포함하지 않는다.
