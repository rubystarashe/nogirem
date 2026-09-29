# Project Explain Refresh 최종 보고

- feature_impact: none — 제품 동작·계약은 변경하지 않고 Rust 코드 구조의 분석·시각화 artifact만 보강
- feature_map: no_change — 정본 capability와 journey 설명은 그대로 유효

## 결과

- `REQ-EXPLAIN-01`: 충족. `docs/explain/20260929-1835-rust-full/`에 HTML 네 개와 manifest를 생성했다.
- `REQ-EXPLAIN-02`: 충족. 쉬운 설명·기술 구조·시각화가 동일한 GUI → service → manager → worker/helper → Windows 모델을 사용한다.
- `REQ-EXPLAIN-03`: 충족. 핵심 주장에 로컬 source 링크를 제공하고 사실·추정·미검증을 구분했다.
- `REQ-EXPLAIN-04`: 충족. 링크, offline, SVG 접근성, 밝은 배경, 모바일, 인쇄, 실제 브라우저 렌더링을 검증했다.
- `REQ-EXPLAIN-05`: 충족. Rust/Dioxus 현재 구조만 분석했고 Electron 구현은 범위에서 제외했다.
- `REQ-EXPLAIN-06`: 충족. wiki·README를 변경하지 않고 두 정본 불일치를 manifest와 보고서에 기록했다.
- `REQ-EXPLAIN-07`: 충족. 130개 파일에서 생성한 857-node 지식 그래프를 `understanding-report.html` 첫 화면에 내장하고 layer 탐색, 검색, 파일 상세 subgraph, Inspector와 10단계 guided tour를 standalone으로 제공한다.
- 후속 분석: 복구를 완전한 원상복구로 오해하지 않도록 CPU affinity, TCP autotuning, GPU 설정, 설치형 rollback의 확인된 한계를 source link와 함께 보강했다.

## 산출물

- `docs/explain/20260929-1835-rust-full/index.html`
- `docs/explain/20260929-1835-rust-full/eli5-report.html`
- `docs/explain/20260929-1835-rust-full/understanding-report.html`
- `docs/explain/20260929-1835-rust-full/visual-report.html`
- `docs/explain/20260929-1835-rust-full/report-manifest.json`
- `.ua/knowledge-graph.json`
- `.ua/fingerprints.json`
- `.ua/intermediate/scan-result.json`

## 검증

- 로컬 링크·offline·sourceFiles·HTML 계약: PASS
- inline SVG 수와 접근성 metadata: PASS
- 데스크톱 브라우저와 accessibility snapshot: PASS
- 모바일 viewport와 전체 페이지 가로 넘침: PASS
- 실제 390px CSS viewport 후속 검사에서 ELI5 code link 넘침을 발견해 수정 후 재검증: PASS
- print media: PASS
- IDE lint: 오류 없음
- Understand Anything graph deterministic validation: issue 0, 857 nodes, 1,296 edges, 9 layers, 10 tour steps
- 브라우저 상호작용: layer·`main.rs` 상세 graph·Inspector·guided tour PASS
- 390px 반응형: page width 390px, graph canvas만 900px 내부 스크롤, Inspector 단일 열 PASS
- 실제 앱 기능 테스트: 이번 문서 생성 범위에서 실행하지 않음

## 검토

- initial mode: `self`
- final mode: `independent`
- reviewer: `subagent:2b5ebc36-66e3-4832-8d0c-71b10fc193c6`
- verdict: `APPROVED`
- 기록: `reviews/REV-EXPLAIN-01/review-04.md`
- 범위: 생성된 HTML, manifest, run 기록, handoff
- 첫 브라우저 검증에서 세 보고서의 명시적 흰 배경 누락을 발견해 수정하고 재검증했다.
- 후속 검토: `REV-EXPLAIN-FOLLOWUP`, `target-02.sha256`, `APPROVED`
- 후속 첫 round에서 Dioxus `0.7.3`을 실제 resolution으로 단정한 finding을 받아 앱 `0.4.1`, Cargo 요구값 `^0.7.3`, lock `0.7.10`으로 구분하고 재승인받았다.
- 최종 staged snapshot: `REV-EXPLAIN-FOLLOWUP-FINAL`, blob 18개 hash 일치, `APPROVED`
- Understand Anything graph assemble 재검토: 첫 round에서 batch 3·8 한국어 summary 246개 인코딩 손상을 발견해 UTF-8로 재생성했고 857 nodes/1,296 edges·130-file coverage 기준 `APPROVED`
- 최종 graph/viewer 검토: `REV-EXPLAIN-GRAPH-TARGET-01`은 provenance·상태 기록·검색 combobox 접근성 finding으로 `CHANGES_REQUIRED`였다. 수정한 `TARGET-02`의 21개 hash 일치와 ISSUE-01~03 closure를 독립 재검토해 `APPROVED`받았다.
- 최종 bookkeeping snapshot: core graph/viewer hash가 TARGET-02와 동일한 `TARGET-03` 26개 file hash 일치 기준 독립 finish review `APPROVED`.
- 최종 immutable core snapshot: mutable run index와 분리한 `TARGET-04` 10개 file hash 일치 기준 독립 finish review `APPROVED`.

## 남은 사항

- 정본 불일치 수정은 `--sync-wiki`가 없어 수행하지 않았다.
- 실제 게임·GPU·녹화·설치·업데이트 QA 공백은 보고서에 미검증으로 유지했다.
- 확인된 복구 한계는 문서화했으며 동작 자체를 변경하지 않았다.
- 공식 Understand Anything dashboard는 prebuilt viewer bind `EACCES`와 Node 23/Vite 6 package import 호환 오류로 `BLOCKED`다. 동일 graph를 내장한 standalone viewer는 실제 브라우저로 검증했으며 공식 dashboard 성공으로 표현하지 않는다.
- 정적 import가 없는 설정·문서·자산 53개는 orphan warning으로 남아 있으나 node 누락·dangling edge·layer 미배치는 없다.
- 현재 embedded graph에는 `<`나 closing-script 문자열이 없고 graph와 의미상 동일하다. closing-script 악성 fixture를 넣은 생성기 회귀 테스트는 실행하지 않아 비차단 limit로 남긴다.
