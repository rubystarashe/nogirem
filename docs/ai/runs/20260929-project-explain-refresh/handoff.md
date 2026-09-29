# Project Explain Refresh Handoff

- Updated: 2026-09-29 19:29:00 +09:00
- Run ID: `20260929-project-explain-refresh`
- 목표: 현재 Rust/Dioxus 구조를 신규 개발자용 독립 HTML 보고서로 설명
- 상태: 후속 사실 보강·브라우저 QA·독립 재검토 완료, local commit 준비

## 완료

- 현재 Rust/Dioxus 앱 경로를 분석하고 외부 SDK·vendored framework·Electron 앱 구현·생성물을 제외했다.
- HTML 네 개와 manifest를 `docs/explain/20260929-1835-rust-full/`에 생성했다.
- 링크, offline, SVG 접근성, 모바일, 인쇄, 실제 브라우저 렌더링을 통과했다.
- `DIOXUS_MIGRATION.md`의 역사적 중간 상태와 `README.md` 설치 자산명 불일치를 기록했다.
- wiki·README는 `--sync-wiki`가 없어 변경하지 않았다.
- 완료된 기능 분석을 source와 대조해 CPU affinity, TCP autotuning, GPU 설정, 설치형 rollback의 복구 한계를 ELI5·기술 보고서·manifest에 보강했다.
- 앱 `0.4.1`, Cargo Dioxus 요구값 `^0.7.3`, lock resolution `0.7.10`을 구분하고 WebView2 경로와 affinity worker 수명을 명확히 했다.
- 실제 390px CSS viewport에서 ELI5의 긴 code link overflow를 발견해 수정했다.

## 제한

- 실제 앱 build/test와 시스템 변경 QA는 실행하지 않았다.
- 초기 self-review 뒤 독립 검토에서 `target-03.sha256` 기준 승인을 받았다.
- 후속 target은 `REV-EXPLAIN-FOLLOWUP/target-02.sha256` 기준 독립 재검토 승인을 받았다. 첫 round의 Dioxus 버전 단정 finding은 요구 범위와 lock resolution을 분리해 닫았다.
- 브라우저 screenshot은 Cursor browser 검증 artifact로 남겼고 측정값과 snapshot 요약은 `evidence/browser-verification.md`에 기록했다.

## 다음 행동

1. 필요하면 별도 `--sync-wiki` 실행으로 확인된 정본 불일치를 수정한다.
2. 코드 구조가 바뀐 뒤에는 이 manifest의 target commit을 기준으로 incremental refresh를 실행한다.

## 링크

- 요청: `request.md`
- 계획: `plan.md`
- QA: `qa.md`
- 결과: `final-report.md`
- 보고서: `../../../explain/20260929-1835-rust-full/index.html`
