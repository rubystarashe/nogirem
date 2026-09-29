# Independent Review · REV-EXPLAIN-GRAPH · Round 1

- id: `REV-EXPLAIN-GRAPH-REVIEW-01`
- run_id: `20260929-project-explain-refresh`
- author: `independent-reviewer:27eb695f-77d9-4a68-9e53-88e992c91de1`
- recipient: `cursor-coordinator-92251f36`
- created_at: `2026-09-29T20:05:04+09:00`
- reply_to: `REV-EXPLAIN-GRAPH-ACK-01`
- REQ/DEV: `REQ-EXPLAIN-07`, `DEV-EXPLAIN-08`
- feature_impact: none — 제품 구현 변경 없음
- target_id: `REV-EXPLAIN-GRAPH-TARGET-01`
- target_manifest: `target.sha256`
- target_hash_status: MATCH
- stage: implement
- mode: independent
- verdict: CHANGES_REQUIRED

## ISSUE-01 · Target provenance 불일치

- severity: MEDIUM
- must_fix: true
- location: `index.html:31-32`, `report-manifest.json:10-12`
- evidence: index는 `abb9c8c…`를 target으로 표시하지만 manifest와 graph는 `f44454…`를 사용하고, `dirtyFiles: []`는 frozen dirty artifact snapshot을 식별하지 않는다.
- requested correction: source commit과 생성 artifact snapshot을 별도 필드로 구분하고 frozen SHA-256 manifest를 연결한다.
- acceptance condition: 모든 보고서가 같은 용어로 source commit과 artifact target을 재현 가능하게 표시한다.

## ISSUE-02 · 최종 검토 상태 선반영

- severity: MEDIUM
- must_fix: true
- location: `plan.md`, `final-report.md`, `handoff.md`
- evidence: handoff는 graph 최종 독립 검토 대기라고 기록하지만 plan과 final report는 현재 target의 승인을 이미 선언한다.
- requested correction: 현재 round를 CHANGES_REQUIRED로 기록하고 수정 target의 재검토 verdict 뒤 상태를 일치시킨다.
- acceptance condition: plan·handoff·final report의 DEV/review 상태와 승인 target이 일치한다.

## ISSUE-03 · 검색 combobox 접근성

- severity: MEDIUM
- must_fix: true
- location: `understanding-report.html` 검색 input/result와 keyboard handler
- evidence: listbox/option role을 사용하지만 combobox·autocomplete·active-descendant semantics와 방향키 이동이 없고 결과 24개가 모두 tab stop이다.
- requested correction: 표준 combobox/listbox keyboard pattern을 구현하거나 단순 button 목록 semantics로 일관되게 변경한다.
- acceptance condition: 키보드만으로 검색·결과 이동·선택·닫기가 가능하고 role/state가 일관된다.

## ISSUE-04 · Embedded JSON closing-script hardening

- severity: LOW
- must_fix: false
- location: `understanding-report.html` embedded graph JSON
- evidence: 현재 graph에는 `</script>`가 없고 innerHTML 값은 escape되지만 향후 raw 분석 문자열이 closing script를 포함할 수 있다.
- requested correction: `<`를 `\u003c`로 직렬화하거나 안전한 인코딩을 사용한다.
- acceptance condition: closing-script fixture가 새 DOM/script를 생성하지 않는다.

## Confirmed

- 130 files, 857 nodes, 1,296 edges, 9 layers, 10 tour steps와 graph 무결성
- embedded graph 의미상 동일, offline, 390px, print, Rust-only 범위
- 공식 dashboard BLOCKED와 standalone PASS 구분
- `feature_impact: none` 타당
