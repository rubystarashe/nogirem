# 브라우저 검증 증거

- 실행 시각: 2026-09-29 18:49~18:53 +09:00
- 서버: 저장소 root에서 `python -m http.server 8765 --bind 127.0.0.1`
- 브라우저: Cursor Chromium browser
- 대상: `docs/explain/20260929-1835-rust-full/`

## 정적 계약 검사

PowerShell로 HTML 4개를 읽어 다음 조건을 검사했다.

- doctype, `lang=ko`, UTF-8, viewport, title
- 모바일·인쇄 media query
- 모든 로컬 `href`의 대상 존재
- 외부 `http(s)` script·stylesheet·asset 없음
- inline SVG 두 개와 각 `role=img`, `title`, `desc`
- manifest sourceFiles 39개 존재

결과:

```json
{"reports":4,"links":"PASS","offline":"PASS","responsiveCss":"PASS","printCss":"PASS","svgAccessibility":"PASS","sources":39}
```

최종 재검사:

```json
{"html":4,"manifest":"PASS","links":"PASS","offline":"PASS","browser":"PASS","responsive":"PASS","sources":39}
```

## 데스크톱 접근성 snapshot

### index

- title: `Nogirem Rust/Dioxus 구조 설명`
- heading level 1: `마비노기 렘 부스터는 어떻게 움직이는가`
- navigation links: 쉬운 설명, 기술 구조, 시각화
- 주요 region: 세 가지 읽는 방법, 분석 범위, 빠른 시작 파일

### visual

- title: `시각화 — Nogirem Rust/Dioxus`
- heading level 1: `Nogirem 실행 구조와 안전한 교체 흐름`
- 주요 region: 런타임 아키텍처, 서명 업데이트와 rollback 흐름, 시각화 근거 파일
- SVG 두 개 모두 DOM에서 `role=img`, `title`, `desc` 확인

### understanding

- title: `기술 구조 — Nogirem Rust/Dioxus`
- heading level 1: `책임 경계, 호출 흐름, 권한과 복구`
- heading level 2 열 개와 source link 58개를 접근성 tree에서 확인

### ELI5

- title: `ELI5 — Nogirem Rust/Dioxus`
- heading level 1: `컴퓨터 안의 작은 호텔 운영팀`
- 주요 region: 호텔 비유, 사용자 흐름, 기능, 업데이트, 비유 한계, 문서 주의점

## 반응형 CDP 측정

### visual · 390×844 emulation

```json
{
  "innerWidth": 390,
  "scrollWidth": 390,
  "background": "rgb(255, 255, 255)",
  "svgs": [
    {"role": "img", "title": true, "desc": true},
    {"role": "img", "title": true, "desc": true}
  ],
  "diagramOverflow": ["auto", "auto"]
}
```

### understanding · 390×844 emulation

```json
{
  "innerWidth": 390,
  "scrollWidth": 390,
  "background": "rgb(255, 255, 255)",
  "tables": [
    {"client": 358, "scroll": 536},
    {"client": 358, "scroll": 623},
    {"client": 358, "scroll": 451}
  ]
}
```

페이지 전체 가로 넘침은 없고 표 자체가 내부 가로 스크롤을 제공한다.

### ELI5 · mobile emulation

```json
{
  "innerWidth": 435,
  "scrollWidth": 435,
  "background": "rgb(255, 255, 255)",
  "fontSize": "17px"
}
```

## 인쇄 CDP 측정

```json
{
  "media": true,
  "nav": "none",
  "bodyFont": "14.6667px",
  "width": 390
}
```

## Screenshot artifact 이름

- `nogirem-explain-index-desktop.png`
- `nogirem-explain-visual-desktop.png`
- `nogirem-explain-visual-mobile.png`

Screenshot binary는 Cursor browser artifact에 남고 저장소에는 복사하지 않았다. 첫 visual screenshot에서 body 배경 누락을 발견했으며 세 보고서에 `background:#fff`를 추가한 뒤 재캡처와 CDP 측정을 통과했다.

## 후속 사실·반응형 재검증

- 실행 시각: 2026-09-29 19:13~19:20 +09:00
- 목적: 기능 분석 완료 뒤 복구 한계 보강과 실제 390px CSS viewport 재검증
- 정적 결과: HTML 4개, sourceFiles 46개, 로컬 링크, offline asset, SVG 접근성 모두 PASS
- 접근성 snapshot: 기술 보고서의 `확인된 복구 한계` heading과 네 항목, ELI5의 완전한 원상복구 제한 문구와 source link를 확인
- 기술 보고서: 390×844에서 document/body 폭 390, 흰 배경, 표 세 개만 내부 가로 스크롤
- 개요: 390×844에서 viewport 390, document/body 폭 375, 흰 배경
- 시각화: 390×844에서 document/body 폭 375, SVG diagram 두 개만 `overflow-x:auto`로 812px 내부 스크롤
- ELI5: 첫 390px CSS viewport 검사에서 긴 inline code link가 document 폭을 435px로 넓히는 문제를 발견했다. <code>에 `overflow-wrap:anywhere`와 `word-break:break-word`를 추가한 뒤 viewport 390, document/body 폭 375로 재검증했다.
- 독립 검토 후 정정: 앱 버전 `0.4.1`, Dioxus 선언 요구값 `^0.7.3`, `Cargo.lock` resolution `0.7.10`을 구분하고 sourceFiles에 `desktop/Cargo.lock`을 추가했다.

## Understand Anything 인터랙티브 graph 재검증

- 실행 시각: 2026-09-29 20:00~20:05 +09:00
- source target: `f44454cab129a0dfe3be8f55b8e9e95475ec4f35`
- graph: 130 files, 857 nodes, 1,296 edges, 9 layers, 10 tour steps
- 페이지: `understanding-report.html`
- 접근성 snapshot: 9개 layer button, node 검색, 5개 type filter, tour control, keyboard-selectable SVG node, Inspector, 기존 Reference 영역 확인
- 상호작용: `main.rs` file node 선택 후 contained function과 import 대상 subgraph, summary/tag/path와 incoming/outgoing 관계 표시 확인
- guided tour: 첫 단계 `제품 목적과 범위`가 `README.md` node만 bounded graph로 표시하고 Inspector와 설명을 갱신함을 확인
- 검색 접근성: `service` 검색 결과 30개의 option tab stop이 0임을 확인했다. ArrowDown 뒤 input focus를 유지한 채 `search-option-0`이 active descendant와 selected 상태가 됐고 Enter 뒤 listbox가 닫히며 `app_services.rs` 상세 graph와 Inspector가 열렸다.
- offline: 외부 script·stylesheet·asset·`fetch()` 없이 embedded `knowledge-graph-data` JSON을 사용

390×844 emulation 측정:

```json
{
  "innerWidth": 390,
  "documentWidth": 390,
  "bodyWidth": 390,
  "graphClientWidth": 368,
  "graphScrollWidth": 900,
  "workspaceDisplay": "block",
  "inspectorWidth": 368
}
```

- 결과: 페이지 전체 가로 넘침 없음, graph canvas만 내부 가로 스크롤, Inspector 한 열 배치
- screenshot artifact: `nogirem-understand-graph-mobile.png`, `nogirem-understand-graph-desktop.png`
- 공식 dashboard 실행: prebuilt viewer의 `127.0.0.1:5173` bind `EACCES`와 source dashboard의 Node 23/Vite 6 package import 오류로 `BLOCKED`. standalone viewer 브라우저 검증 결과와 구분해 기록했다.
