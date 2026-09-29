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
