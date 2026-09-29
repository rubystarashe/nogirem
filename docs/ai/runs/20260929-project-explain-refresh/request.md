# Project Explain Refresh 요청

- Run ID: `20260929-project-explain-refresh`
- 대상: `nogirem`
- 독자: 신규 개발자
- 언어: 한국어
- 방식: `full`
- 기준 commit: `abb9c8c0122dff3355776454aa58201f4cedb055`

## 요구사항

- `REQ-EXPLAIN-01`: 새 timestamp 디렉터리에 독립 실행 가능한 HTML 네 개와 manifest를 생성한다.
- `REQ-EXPLAIN-02`: 쉬운 설명, 기술 구조, 최소 두 개의 inline SVG가 같은 사실과 용어를 사용한다.
- `REQ-EXPLAIN-03`: 모든 핵심 주장에 저장소 상대 경로 근거를 연결하고 사실·추정·미검증을 구분한다.
- `REQ-EXPLAIN-04`: 외부 CDN·원격 script·추적 코드를 사용하지 않고 로컬 링크, 모바일, 인쇄, 브라우저 렌더링을 검증한다.
- `REQ-EXPLAIN-05`: 사용자 지시에 따라 현재 Rust/Dioxus 버전만 분석하고 Electron 애플리케이션 구현은 제외한다.
- `REQ-EXPLAIN-06`: `--sync-wiki`가 없으므로 정본 문서는 변경하지 않고 발견한 불일치만 기록한다.

## 범위 제외

- `release/`, build target, vendored Dioxus, 외부 ADLX SDK
- Electron 애플리케이션 내부 구현
- 실제 Windows 최적화·설치·업데이트·녹화·드라이버 변경 QA
