# Project Explain Refresh Handoff

- Updated: 2026-09-29 20:25:00 +09:00
- Run ID: `20260929-project-explain-refresh`
- 목표: 현재 Rust/Dioxus 구조를 신규 개발자용 독립 HTML 보고서로 설명
- 상태: Understand Anything 지식 그래프·standalone viewer·브라우저 QA·`TARGET-02` 독립 재검토 승인 완료, local commit 준비
- feature_impact: none — 제품 동작이 아닌 코드 구조 분석·시각화 artifact
- feature_map: no_change — 정본 기능 지도 변경 없음
- 현재 target: source commit `f44454cab129a0dfe3be8f55b8e9e95475ec4f35`와 이번 dirty graph/report snapshot

## 완료

- 현재 Rust/Dioxus 앱 경로를 분석하고 외부 SDK·vendored framework·Electron 앱 구현·생성물을 제외했다.
- HTML 네 개와 manifest를 `docs/explain/20260929-1835-rust-full/`에 생성했다.
- 링크, offline, SVG 접근성, 모바일, 인쇄, 실제 브라우저 렌더링을 통과했다.
- `DIOXUS_MIGRATION.md`의 역사적 중간 상태와 `README.md` 설치 자산명 불일치를 기록했다.
- wiki·README는 `--sync-wiki`가 없어 변경하지 않았다.
- 완료된 기능 분석을 source와 대조해 CPU affinity, TCP autotuning, GPU 설정, 설치형 rollback의 복구 한계를 ELI5·기술 보고서·manifest에 보강했다.
- 앱 `0.4.1`, Cargo Dioxus 요구값 `^0.7.3`, lock resolution `0.7.10`을 구분하고 WebView2 경로와 affinity worker 수명을 명확히 했다.
- 실제 390px CSS viewport에서 ELI5의 긴 code link overflow를 발견해 수정했다.
- 공식 Understand Anything v2.9.0 pipeline으로 Rust 앱 범위 130개 파일을 분석해 857 nodes, 1,296 edges, 9 layers, 10 tour steps를 `.ua/knowledge-graph.json`에 저장했다.
- 첫 assemble review에서 batch 3·8의 한국어 summary 246개 인코딩 손상을 발견해 UTF-8 without BOM으로 재생성하고 issue 0으로 재승인받았다.
- `understanding-report.html`을 기술 시각화 중심으로 재구성했다. layer selector, node 검색, type filter, 파일 내부 function/class graph, incoming/outgoing Inspector, guided tour를 graph JSON과 함께 standalone으로 내장했다.
- 실제 브라우저에서 `main.rs` 상세 graph와 tour 첫 단계를 조작하고 390px에서 page 390px, graph 내부 900px 스크롤, Inspector 단일 열을 확인했다.

## 제한

- 실제 앱 build/test와 시스템 변경 QA는 실행하지 않았다.
- 초기 self-review 뒤 독립 검토에서 `target-03.sha256` 기준 승인을 받았다.
- 후속 target은 `REV-EXPLAIN-FOLLOWUP/target-02.sha256` 기준 독립 재검토 승인을 받았다. 첫 round의 Dioxus 버전 단정 finding은 요구 범위와 lock resolution을 분리해 닫았다.
- 브라우저 screenshot은 Cursor browser 검증 artifact로 남겼고 측정값과 snapshot 요약은 `evidence/browser-verification.md`에 기록했다.
- 공식 dashboard 자체는 prebuilt viewer의 localhost bind `EACCES`와 Node 23/Vite 6 package import 오류로 `BLOCKED`다. standalone viewer PASS와 구분하며 같은 실행 경로를 더 반복하지 않는다.
- 정적 import가 없는 53개 설정·문서·자산 node는 orphan warning으로 남아 있다.
- `REV-EXPLAIN-GRAPH-TARGET-01`은 source/artifact target 구분, review 상태 선반영, 검색 combobox 접근성 finding으로 `CHANGES_REQUIRED`였고 모두 수정해 재검토를 준비했다.
- `REV-EXPLAIN-GRAPH-TARGET-02`는 21개 frozen file hash 일치와 ISSUE-01~03 closure 기준 승인됐고, 상태·증거·index를 포함한 최종 `TARGET-03` 26개 file snapshot도 독립 finish review에서 `APPROVED`다. ISSUE-04의 악성 closing-script fixture 미실행은 비차단 limit로 남는다.
- 이후 mutable run index와 분리한 최종 core `TARGET-04` 10개 file snapshot도 hash 일치 기준 독립 `APPROVED`다.

## 다음 행동

1. 승인된 명시적 파일만 stage해 local checkpoint commit을 만든다.
2. 필요하면 별도 `--sync-wiki` 실행으로 확인된 정본 불일치를 수정한다.
3. 코드 구조가 바뀐 뒤에는 `.ua/meta.json`의 target commit을 기준으로 incremental refresh를 실행한다.

## 링크

- 요청: `request.md`
- 계획: `plan.md`
- QA: `qa.md`
- 결과: `final-report.md`
- 보고서: `../../../explain/20260929-1835-rust-full/index.html`
- 지식 그래프: `../../../../.ua/knowledge-graph.json`
