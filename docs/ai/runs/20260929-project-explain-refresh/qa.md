# Project Explain Refresh QA

- feature_impact: none — 제품 기능을 실행하거나 변경하지 않고 구조 시각화 artifact만 검증
- feature_map: no_change — durable feature record 변경 없음

## 환경

- Windows 10 x64
- Chromium 기반 Cursor browser
- 대상 commit: `abb9c8c0122dff3355776454aa58201f4cedb055`
- 산출물: `docs/explain/20260929-1835-rust-full/`

## 결과

### QA-EXPLAIN-01 · 파일과 계약

- 연결: REQ-01, REQ-03, REQ-04
- 절차: HTML 네 개와 manifest 존재, doctype, `lang=ko`, UTF-8, viewport, title, 모바일/인쇄 CSS, sourceFiles 존재 여부를 검사했다.
- 기대: 누락과 깨진 source 경로가 없다.
- 실제: HTML 4개, sourceFiles 39개 모두 통과했다.
- 상태: `PASS`

### QA-EXPLAIN-02 · 링크와 offline

- 연결: REQ-03, REQ-04
- 절차: 모든 상대 `href`를 보고서 디렉터리 기준으로 해석하고 파일 존재를 검사했다. `http(s)` script·stylesheet·asset 참조를 검사했다.
- 기대: 깨진 로컬 링크와 외부 실행 의존성이 없다.
- 실제: `links=PASS`, `offline=PASS`.
- 상태: `PASS`

### QA-EXPLAIN-03 · SVG 접근성

- 연결: REQ-02, REQ-04
- 절차: 시각화 보고서에 SVG가 두 개 이상인지, 각각 `role=img`, `aria-labelledby`, `title`, `desc`를 갖는지 검사했다.
- 기대: 그림 두 개가 독립 설명과 텍스트 범례를 제공한다.
- 실제: 아키텍처·업데이트 흐름 SVG 모두 통과했다.
- 상태: `PASS`

### QA-EXPLAIN-04 · 데스크톱 브라우저

- 연결: REQ-04
- 절차: index와 visual report를 실제 브라우저로 열고 accessibility snapshot과 full-page screenshot을 확인했다.
- 기대: 밝은 배경, 읽을 수 있는 대비, 정상 heading/link 구조, SVG 렌더링.
- 실제: 첫 visual 렌더링에서 명시적 body 배경 누락을 발견했다. visual·ELI5·understanding에 흰 배경을 추가한 뒤 재검증에 통과했다.
- 증거 이름: `nogirem-explain-index-desktop.png`, `nogirem-explain-visual-desktop.png`
- 측정·snapshot 요약: `evidence/browser-verification.md`
- 상태: `PASS`

### QA-EXPLAIN-05 · 모바일과 인쇄

- 연결: REQ-04
- 절차: 390×844 device emulation에서 세 보고서를 열고 `documentElement.scrollWidth`, body background, 표·SVG overflow를 검사했다. print media에서 navigation 숨김과 본문 크기를 검사했다.
- 기대: 페이지 전체 가로 넘침이 없고 복잡한 표·SVG만 내부 가로 스크롤을 제공하며 인쇄 navigation이 숨겨진다.
- 실제: visual 390/390, understanding 390/390, ELI5 viewport와 document 폭 일치. 흰 배경, SVG 내부 `overflow-x:auto`, print navigation `display:none`을 확인했다.
- 증거 이름: `nogirem-explain-visual-mobile.png`
- 측정값: `evidence/browser-verification.md`
- 상태: `PASS`

### QA-EXPLAIN-06 · 후속 사실·반응형 재검증

- 연결: REQ-02, REQ-03, REQ-04
- 절차: 완료된 기능 분석의 복구 관련 주장을 실제 Rust source와 대조하고, 보강한 ELI5·기술 보고서의 링크·offline 계약·접근성 snapshot·390×844 CSS viewport를 다시 검사했다.
- 기대: 복구 범위를 과장하지 않고 모든 source link가 존재하며 페이지 전체 가로 넘침이 없다.
- 실제: CPU affinity, TCP autotuning, GPU 설정, 설치형 rollback의 제한을 source에서 확인했다. HTML 4개와 sourceFiles 46개 검사가 통과했다. ELI5의 긴 code link가 만드는 435px 넘침을 발견해 wrapping CSS를 추가했고 viewport 390, document/body 폭 375로 재검증했다. 기술 보고서는 document/body 폭 390, 시각화는 page 폭 375와 diagram 내부 스크롤만 확인했다. 독립 검토에서 Dioxus 요구값과 lock resolution의 구분 누락을 발견해 각각 `^0.7.3`과 `0.7.10`으로 바로잡았다.
- 증거: `evidence/browser-verification.md`
- 상태: `PASS`

### QA-EXPLAIN-07 · 인터랙티브 코드 지식 그래프

- 연결: REQ-EXPLAIN-07, DEV-EXPLAIN-08
- 대상: source target `f44454cab129a0dfe3be8f55b8e9e95475ec4f35`, `.ua/knowledge-graph.json`, `understanding-report.html`
- 환경: Windows 10 x64, Node 23.11.1, Understand Anything v2.9.0, Cursor Chromium
- 사전 조건: Rust/Dioxus 앱 범위만 포함하고 `/src`, `/test`, Electron 본체, `release/target/vendor/third_party`, 과거 run을 `.ua/.understandignore`로 제외
- 절차: 공식 scan·semantic batch·구조 추출·merge·layer·tour·validator·fingerprint pipeline을 실행하고, 내장 JSON parse와 node/edge 참조를 검사했다. 브라우저에서 layer 전환, `main.rs` 파일 상세 subgraph, Inspector incoming/outgoing 관계, guided tour 첫 단계를 조작했다. 390×844 emulation에서 페이지 폭과 graph 내부 overflow를 측정했다.
- 기대: 실제 코드 graph가 기술 보고서 첫 화면의 중심이며, 130개 파일이 layer에 빠짐없이 배치되고 검색·상세·tour가 외부 의존성 없이 동작한다. 모바일은 페이지 전체가 넘치지 않고 graph canvas만 내부 가로 스크롤한다.
- 실제: 130 files, 857 nodes, 1,296 edges, 9 layers, 10 tour steps. validation issue 0, layer 누락·중복 0, tour dangling node 0. `main.rs` 선택 시 함수 3개와 import 7개를 포함한 상세 graph와 outgoing 10개가 표시됐고 tour 1/10은 `README.md`를 강조했다. 모바일 측정은 viewport/document/body 390px, graph client 368px, graph scroll 900px, workspace `display:block`, Inspector 368px이었다.
- 접근성 rework: 독립 검토 finding에 따라 검색을 combobox/listbox pattern으로 변경했다. 30개 option의 tab stop 0, ArrowDown 뒤 `aria-activedescendant=search-option-0`·`aria-selected=true`·input focus 유지, Enter 뒤 listbox 닫힘과 `app_services.rs` 상세 graph 전환을 실제 브라우저에서 확인했다.
- UTF-8 rework: 첫 assemble review에서 batch 3·8의 한국어 summary 246개가 `?`로 손상된 것을 발견해 UTF-8 without BOM으로 재생성했고 독립 재검토에서 issue 0으로 닫았다.
- 독립 최종 검토: `REV-EXPLAIN-GRAPH-TARGET-02` 21개 frozen file hash 일치, ISSUE-01~03 closure, verdict `APPROVED`.
- standalone viewer verdict: `PASS`
- 공식 dashboard verdict: `BLOCKED` — prebuilt viewer는 `127.0.0.1:5173` bind `EACCES`, source dashboard는 Node 23/Vite 6의 `#module-sync-enabled` package import 오류였다. 네 번째 실행 시도 뒤 중단했으며 graph 생성·standalone viewer 검증에는 영향이 없다.
- browser evidence: `nogirem-understand-graph-mobile.png`, `nogirem-understand-graph-desktop.png`
- 상태: `PASS`

## 실행하지 않은 검증

- 앱 build/test, 실제 최적화, 설치, 업데이트, driver, 녹화 QA는 보고서 생성 변경과 직접 관련이 없어 실행하지 않았다.
- 기존 기능 검증 결과는 보고서에서 저장소 문서와 테스트 범위로 인용했으며 이번 실행 결과로 표현하지 않았다.
- 공식 Understand Anything dashboard 자체 실행은 환경 제약으로 `BLOCKED`이며 standalone viewer의 실제 브라우저 실행으로 대체했다. 이를 공식 dashboard PASS로 표현하지 않는다.
