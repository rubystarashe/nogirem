# 0.4.5 공개 배포 증거

- evidence_id: `EVIDENCE-RELEASE-045-PUBLICATION`
- producer: `Cursor Agent`
- created_at: `2026-10-05T12:22:00Z`
- run_id: `20261005-release-045`
- checkpoint_id: `CP-RELEASE-045`
- source_target: `2fd708bee4af0513a68b21618e81e0173516c098`
- execution_target: `Windows 10.0.26200 x64, Node >=20, Rust locked workspace, GitHub Release v0.4.5`
- release_url: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.5`
- dependency_mode: `실제 GitHub origin·Release, 실제 NSIS·Microsoft WebView2 bootstrapper, 실제 Ed25519 signing`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — 0.4.5 실시간 게임 서버 모니터링 공개`
- feature_map: `no_change — 기존 ACTIVE entry와 source 동작 설명이 정확함`
- architecture_impact: `기존 signed package·update routing·Electron migration bridge 공개 경계 실행`
- architecture_contract: `no_change — 기존 경계 변경 없음`

## Checks

- `PASS` — `npm test`: 204/204.
- `PASS` — `npm run test:desktop`: 8/8.
- `PASS` — `npm run test:backend-integration`: memory worker, affinity worker, TCP read-only operation.
- `PASS` — `npm run package:win`: release build, packaged native contract, Rust-only payload, Microsoft bootstrapper Authenticode, NSIS `/WX`, installer size budget, signed manifests.
- `PASS` — release assembly: 자산 10개, installer·portable alias byte identity, Electron 0.3.19 bridge·blockmap·`latest.yml`.
- `PASS` — origin/main과 v0.4.5 tag가 source target을 가리킴.
- `PASS` — 공개 Release는 draft·prerelease가 아니며 자산 10개를 제공함.
- `PASS` — 공개 자산 10/10 재다운로드 SHA-256이 후보와 일치함.
- `PASS` — 공개 다운로드를 입력으로 `prepare-release.mjs`를 다시 실행해 Ed25519 payload와 Electron bridge를 검증함.

## Primary artifacts

- installer: `nogirem-dioxus-setup-0.4.5.exe`, `9,563,418 bytes`, SHA-256 `aada494d6321bbcfd8875a2e4e38c32fc571ba0ea20a49399c88b0fc163308b7`
- portable: `nogirem-dioxus-portable-0.4.5.exe`, `7,790,592 bytes`, SHA-256 `5269af6ca82b8f31e80decbe783665ae2ff3f1513c1caceccdbec45dc6c1bef8`
- update manifest: SHA-256 `0d80b2644c7e7e282741c32c043f0c02b17912ae2b3113c7f181315664c14637`
- portable manifest: SHA-256 `8abdc34da030eb75d73ac12565dfb8631289c3db0a995010edd984d71e95b046`

## Limitations

- 실제 사용자 PC 설치·제거·자동 업데이트·포터블 실행은 `NOT_RUN`.
- 최신 게임 전경·물리 입력·혼합 DPI 시각 검증은 `NOT_RUN`.
- raw terminal 출력은 로컬 세션에만 존재하며 이 문서는 핵심 결과와 재현 가능한 target·hash를 보존함.
