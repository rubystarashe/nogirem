# Package evidence

- evidence_id: `EVID-PACKAGE-0411-001`
- run_id: `20261108-package-0411`
- checkpoint_id: `CP-PACKAGE-0411`
- producer: `cursor-agent-92251f36`
- executed_at: `2026-11-07T17:37:53Z..2026-11-07T17:40:56Z`
- source_target: `545ec1767e03745bc598cc90c48f38ed484db987`
- execution_target: `release/dioxus-0.4.11-2026-11-07T17-37-53-373Z`
- environment: `Windows 10.0.26200 x64, locked Rust release build, Node package tool, NSIS 3.0.4.1`
- dependency_mode: `locked Cargo dependencies, current-source native helper rebuild, local Ed25519 release key; private key path와 내용은 기록하지 않음`
- command: `npm run package:win`
- expected: `0.4.11 signed installer·portable, separate manifests, Rust-only payload, NSIS /WX, installer decimal 10MB 미만`
- actual: `PASS, exit 0`
- feature_impact: `FEAT-NOGIREM-DXVK, FEAT-NOGIREM-NETWORK-FASTPING — 0.4.11 payload; FEAT-NOGIREM-UPDATE-LIFECYCLE — signed local artifacts`
- feature_map: `updated — canonical revision 48`
- architecture_impact: `version identity, native helper build, non-solid NSIS, Ed25519 manifest`
- architecture_contract: `updated — revision 52`

## Artifacts

- installer: `nogirem-dioxus-setup-0.4.11.exe`
  - size: `8,024,324`
  - SHA-256: `f2e538efc73b04ae9bae579e3c00931080052e9f925fbf81a0aad911673ef8e6`
- portable: `nogirem-dioxus-portable-0.4.11.exe`
  - size: `7,799,614`
  - SHA-256: `cc1b472eb33f23d3e5e48876edd00ffcc0d9f44c19f96f215726f5b681080880`
- update manifest SHA-256: `2d684a07564f084769687692e6737bc47526930235a219fbf61e52142c968db8`
- portable manifest SHA-256: `b779ae5e0a8345a1f39b167b5613d6cd86639bb2c860be78e9aca88ec236a87f`
- build metadata SHA-256: `63e1d23a6dbfd88f118bd4ff5214abd2e75d8150b3d87c54e8f9bc6ff4c68df0`

## Verification

- `PASS` — package·app version 0.4.11, setup·portable PE ProductVersion 0.4.11.
- `PASS` — embedded Rust service contract, forbidden Node/Electron runtime payload, single recorder helper와 runtime image checks.
- `PASS` — installer `8,024,324 < 10,000,000` bytes.
- `PASS` — update.json과 portable-update.json을 embedded public key로 독립 Ed25519 검증하고 payload version·size·SHA-256을 실제 EXE와 비교.
- `PASS` — package 후 installer·portable·signing contract tests 17/17.
- canonical package command가 committed native source에서 recorder helper를 재생성하므로 tracked helper working copy가 변경됐다. package payload helper는 이 재생성 결과이며 release output과 함께 source commit에 포함하지 않는다.
- evidence limitation: 실제 설치·제거·포터블 실행·이전 버전 자동 업데이트와 reporter PC는 `NOT_RUN`.
