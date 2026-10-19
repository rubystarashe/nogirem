# Signed non-solid package evidence

- evidence_id: `EVID-NONSOLID-PACKAGE-048-001`
- producer: `cursor-agent-92251f36`
- created_at: `2026-10-19T18:16:00Z`
- run_id: `20261020-nonsolid-package-048`
- checkpoint_id: `CP-NONSOLID-PACKAGE-048`
- source_target: `base af244d2ea823dd51b6bd0da9f9d8f81be6f11105 + current owned dirty target`
- execution_target: `release/dioxus-0.4.8-2026-10-19T18-14-22-935Z`
- environment: `Windows 10.0.26200 x64, Node 22.20.0, Rust locked release, NSIS 3.0.4.1, ffmpeg/ffprobe`
- dependency_mode: `locked Rust dependencies, current-source native helpers, local Ed25519 release signing key; signing key content not recorded`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — external WebView2 prerequisite와 non-solid package 전달`
- feature_map: `updated — canonical revision 38, feature revision 11`
- architecture_impact: `single recorder helper, non-solid LZMA, WebView2 external prerequisite와 10MB package gate`
- architecture_contract: `updated — revision 43`

## 결과

- `PASS` — `npm run package:win`, exit 0; NSIS가 installer·portable 모두 `Using lzma compression`으로 non-solid 출력.
- installer: `8,014,317 bytes`, SHA-256 `acb7e89645aa2add439d9e14f59e711cd367e03753314020be17023705bd7745`.
- portable: `7,789,628 bytes`, SHA-256 `b46afe5c9aac5e93c9c02e851707cd670ad398720942784cd470c908f98b3df2`.
- update manifest: `422 bytes`, SHA-256 `6f5a2e4ff8a731afa5f701f0cc31838af9a99be5f60aa94865e409026109f808`.
- portable manifest: `425 bytes`, SHA-256 `aa2ac7ba06d2893cd99c8af36ef2e271e58310ea4c2421865176fd90fe49db1a`.
- `PASS` — Ed25519 installer·portable manifest 생성과 자체 검증.
- `PASS` — package recorder helper 수 `1`, source `bin`과 packaged helper SHA-256 일치, `tools-bin` 부재.
- `PASS` — package output에 `MicrosoftEdgeWebview2Setup.exe` 부재.
- `PASS` — packaged `VERSION_HISTORY.md`가 source와 byte-identical.
- `PASS` — packaged `public/ost.mp3`는 MP3, 44.1kHz stereo, 128kbps, `448,188 bytes`, `27.934104s`.
- `PASS` — targeted package/blackbox/native tests 24/24, Node 213/213, backend 128 PASS·3 ignored와 parity 2/2, desktop 8/8, native integration 3/3.
- `PASS` — 변경 파일 IDE diagnostics 0.
- `NOT_RUN` — 실제 WebView2 미설치 안내 버튼·browser open, 관리자 설치·제거, 포터블 실행, VirusTotal 비교.

## 제한

- 첫 native integration 병렬 실행은 desktop test가 같은 debug executable을 교체하는 동안 memory worker가 exit 1을 반환해 실패했다. 두 작업 완료 후 단독 재실행한 동일 검사 3/3은 통과했으며 최종 근거는 재실행 결과다.
- generated `release/` 산출물은 source commit에 포함하지 않는다.
