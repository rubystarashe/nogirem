# 0.4.6 공개 배포 증거

- evidence_id: `EVID-RELEASE-046-PUBLICATION`
- producer: `Cursor Agent`
- created_at: `2026-10-15T06:40:30Z`
- run_id: `20261015-release-046`
- checkpoint_id: `CP-RELEASE-046`
- source_target: `4838885c2d810b7ec12cd1dfb99a83c0d8fbd21c`
- execution_target: `Windows 10.0.26200 x64, GitHub Release v0.4.6`
- release_url: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.6`
- feature_impact: `FEAT-NOGIREM-CHANNEL-PING — tray 상태 fix 공개; FEAT-NOGIREM-GAME-OVERLAY — REMOVED source와 안전 공지 공개`
- feature_map: `no_change — canonical entries remain accurate`
- architecture_impact: `기존 signed package·update routing·Electron migration bridge·GitHub Release 경계 실행`
- architecture_contract: `no_change — package·update·image recognition 금지 경계 변경 없음`

## Checks

- `PASS` — final `NOTICE.md` Node application notice test와 Rust notice rendering safety test.
- `PASS` — exact source package: release build, packaged native contract, Microsoft WebView2 signature, NSIS `/WX`, installer size budget, Ed25519 manifests.
- `PASS` — release assembly: 자산 10개, installer·portable alias byte identity, Electron 0.3.19 bridge·blockmap·`latest.yml`.
- `PASS` — origin/main과 annotated v0.4.6 tag가 source target을 가리킴.
- `PASS` — 공개 Release는 draft·prerelease가 아니며 자산 10개를 제공함.
- `PASS` — 공개 자산 10/10 재다운로드 SHA-256이 후보와 일치함.
- `PASS` — 공개 다운로드를 입력으로 `prepare-release.mjs`를 다시 실행해 Ed25519 payload와 Electron bridge를 검증함.

## Primary artifacts

- package_directory: `release/dioxus-0.4.6-2026-10-15T06-36-47-151Z`
- release_candidate: `release/ready-v0.4.6-2026-10-15T06-38-18-723Z`
- installer: `nogirem-dioxus-setup-0.4.6.exe`, `9,572,561 bytes`, SHA-256 `01dffc8375f98d93e01f84e7e5137df3fbaa087879090a14e9985813b0678df1`
- portable: `nogirem-dioxus-portable-0.4.6.exe`, `7,791,805 bytes`, SHA-256 `68ff04252b14559ee253ff4ef08c3b8bad1d4d0885e17a6863a534c3b3c56e53`
- update manifest: SHA-256 `de2e880b174d2088d12b76007b830d3daada8d71216a4b6027871de9850c3ef6`
- portable manifest: SHA-256 `071372d05289dfc9d5e743357701c09b962059fb16f74a58d3e83715789ec6d9`
- public_download: `release/public-v0.4.6-2026-10-15T06-40-00Z`, 10/10 hash match

## Limitations

- 실제 사용자 PC 설치·제거·자동 업데이트·포터블 실행은 `NOT_RUN`.
- 실제 게임·혼합 DPI 시각 검증은 `NOT_RUN`.
- package/publication은 `SELF_REVIEW`; 독립 reviewer 없음.
