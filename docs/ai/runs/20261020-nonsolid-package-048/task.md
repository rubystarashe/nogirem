# 0.4.8 non-solid 패키지 오탐 완화

- record_id: `RUN-20261020-NONSOLID-PACKAGE-048`
- run_id: `20261020-nonsolid-package-048`
- checkpoint_id: `CP-NONSOLID-PACKAGE-048`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-19T18:02:00Z`
- source_base: `af244d2ea823dd51b6bd0da9f9d8f81be6f11105`
- request: `recorder helper 중복 통합, bundled WebView2 제거, 음악만 재압축하고 non-solid LZMA를 적용`
- status: `COMPLETED`
- DEV: `DONE`
- QA: `COMPLETE_PASS`
- review_mode: `SELF_REVIEW`
- review_verdict: `APPROVED`
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — 설치형의 WebView2 미설치 흐름을 bundled 자동 설치에서 Microsoft 공식 설치 안내로 변경; 설치형·포터블 payload 동작은 유지`
- feature_map: `no_change — 기존 update/install capability 범위 안의 package 전달 방식 변경이며 feature entry 의미는 유지`
- architecture_impact: `package bootstrap boundary — non-solid LZMA, recorder helper 단일 실행 경로, WebView2 external prerequisite 안내와 10MB gate`
- architecture_contract: `blocked — 구현·package 검증 뒤 canonical package 경계를 갱신해야 함`

## 목표와 범위

- installer와 portable의 `/SOLID lzma`를 non-solid `lzma`로 변경한다.
- 동일 source·build를 사용하는 recorder runtime/utility helper를 `native/recorder-helper/bin/recorder-helper.exe` 하나로 통합한다.
- WebView2 bootstrapper 다운로드·embedding·실행을 제거하고 미설치 시 Microsoft 공식 다운로드 페이지를 안내한 뒤 설치를 중단한다.
- `public/ost.mp3`만 MP3 128kbps stereo·44.1kHz로 재압축한다.
- decimal 10MB installer gate, portable cache identity, update signing과 실제 package 계약을 유지한다.

## 위험과 검증 계획

- risk: `MEDIUM — 설치 prerequisite 흐름과 package payload 구조가 바뀌며 recorder utility 경로를 공유하지만 제품 runtime·durable data writer·API 계약은 변경하지 않음`
- canonical_extension_path: `desktop/*.nsi → scripts/package-dioxus.mjs → native helper canonical bin → package contract tests`
- durable_data: `no_change — installer registry, portable cache marker와 사용자 설정 writer는 그대로 유지`
- security_privacy: `Microsoft HTTPS 공식 페이지만 열며 새 사용자 입력·데이터 전송 없음`
- validation: targeted installer/portable/blackbox tests, Node suite, desktop/backend checks, signed package, payload inventory, 10MB limit, audio metadata와 helper 단일 identity.
- exclusions: VirusTotal 재업로드, GitHub Release 대치, push·tag·배포, Authenticode 구매·서명.
- unrelated_preserved: 비추적 `data/`.

## 구현 결과

- installer·portable `SetCompressor lzma`로 non-solid 전환.
- recorder utility·runtime·legacy compatibility 경로와 package build를 canonical `native/recorder-helper/bin/recorder-helper.exe` 하나로 통합.
- installer의 WebView2 bootstrapper 다운로드·Authenticode 검사·embedding·실행 제거. registry에 runtime이 없으면 Microsoft 공식 HTTPS 다운로드 페이지를 열지 묻고 변경 전에 중단.
- `public/ost.mp3`를 192kbps `672,215 bytes`에서 128kbps `448,188 bytes`로 재압축. duration `27.934104s`, 44.1kHz stereo 유지.
- signed candidate: `release/dioxus-0.4.8-2026-10-19T18-14-22-935Z`.
- installer: `8,014,317 bytes`, SHA-256 `acb7e89645aa2add439d9e14f59e711cd367e03753314020be17023705bd7745`.
- portable: `7,789,628 bytes`, SHA-256 `b46afe5c9aac5e93c9c02e851707cd670ad398720942784cd470c908f98b3df2`.

## QA

- `PASS` — targeted installer·portable·blackbox·native 24/24.
- `PASS` — Node 213/213.
- `PASS` — backend 128 PASS·3 ignored, legacy/native parity 2/2.
- `PASS` — desktop 8/8.
- `PASS` — native integration 3/3 단독 재실행.
- `PASS` — signed package exit 0, Ed25519 manifests, NSIS non-solid, decimal 10MB gate.
- `PASS` — package recorder helper 1개, source/package helper hash 일치, `tools-bin`·WebView2 bootstrapper 부재.
- `PASS` — packaged audio MP3 44.1kHz stereo 128kbps, duration 보존.
- `PASS` — IDE diagnostics 0.
- `NOT_RUN` — 실제 WebView2 미설치 안내·browser open, 관리자 설치·제거, 포터블 실행, VirusTotal 재분석, push·tag·배포.
- evidence: `evidence/package.md`.

## SELF_REVIEW

- reviewed_scope: NSIS prerequisite 분기와 압축, package source/payload, recorder utility·runtime 경로, audio metadata, update signing·10MB gate, canonical 문서.
- finding: 병렬 desktop build 중 native integration의 debug executable이 교체되어 최초 memory worker 검사가 exit 1로 실패했다.
- disposition: build 완료 후 단독 재실행 3/3 PASS. source 결함 증거가 아니며 최종 evidence에 최초 실패와 재실행 조건을 모두 기록했다.
- mandatory_findings: `none`.
- limitations: 동일 agent가 구현과 검토를 수행했으며 독립 reviewer는 참여하지 않았다. VirusTotal과 실제 prerequisite/install lifecycle은 실행하지 않았다.

## 최종 disposition

- feature_gate: `PASS for source/package scope — FEAT-NOGIREM-UPDATE-LIFECYCLE revision 11; 실제 lifecycle gap 명시`.
- architecture_gate: `PASS — revision 43, non-solid·single helper·external WebView2·10MB enforcement 반영`.
- review_qa_gate: `PASS for automated package scope; manual install·VirusTotal은 후속 범위`.
- target_authorization_gate: `PASS — generated package는 local only, push·release·deploy 없음, 비추적 data 보존`.
- next_action: 사용자 승인 시 새 candidate를 VirusTotal에 비교 업로드하거나 별도 배포 checkpoint에서 0.4.8 자산을 대치한다.
