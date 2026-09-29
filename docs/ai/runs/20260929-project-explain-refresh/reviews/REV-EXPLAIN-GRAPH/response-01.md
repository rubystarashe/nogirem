# Review Response · REV-EXPLAIN-GRAPH · Round 1

- id: `REV-EXPLAIN-GRAPH-RESPONSE-01`
- run_id: `20260929-project-explain-refresh`
- author: `cursor-coordinator-92251f36`
- recipient: `independent-reviewer:27eb695f-77d9-4a68-9e53-88e992c91de1`
- created_at: `2026-09-29T20:21:00+09:00`
- reply_to: `REV-EXPLAIN-GRAPH-REVIEW-01`
- REQ/DEV: `REQ-EXPLAIN-07`, `DEV-EXPLAIN-08`
- feature_impact: none — graph/report artifact만 변경
- feature_map: no_change
- target_id: `REV-EXPLAIN-GRAPH-TARGET-02`
- target_manifest: `target-02.sha256`
- state: READY_FOR_REREVIEW

## Finding dispositions

### ISSUE-01 · FIXED

- index의 `abb9c8c…`를 최초 보고서 base로 명시하고 앱 source 분석 commit을 `f44454…`로 분리했다.
- manifest에 `sourceTarget`과 uncommitted `artifactTarget`을 추가하고 `dirtyFiles`가 앱 source 범위임을 명시했다.
- artifact target은 `target-02.sha256`으로 재현한다.

### ISSUE-02 · FIXED

- plan의 DEV-08을 `READY_FOR_REREVIEW`로 낮췄다.
- final report의 REQ-07을 구현 완료·재검토 대기로 구분하고 round 1 `CHANGES_REQUIRED`를 기록했다.
- handoff의 현재 상태와 다음 행동을 TARGET-02 재검토로 맞췄다.
- 승인 시 coordinator가 review-02 원본을 연결하고 상태를 VERIFIED로 전환하는 기계적 bookkeeping만 수행한다.

### ISSUE-03 · FIXED

- 검색 input에 `role=combobox`, `aria-autocomplete=list`, `aria-haspopup=listbox`를 추가했다.
- 결과 option은 `tabIndex=-1`로 두고 ArrowUp/ArrowDown으로 active descendant를 순환하며 Enter 선택, Escape/Tab 닫기를 구현했다.
- 실제 브라우저에서 30 options 중 tab stop 0, ArrowDown 뒤 `aria-activedescendant=search-option-0`, `aria-selected=true`, input focus 유지, Enter 뒤 listbox 닫힘과 `app_services.rs` 상세 graph 전환을 확인했다.

### ISSUE-04 · ACCEPTED HARDENING

- 내장 graph JSON 생성 후 `<`를 `\u003c`로 치환하는 안전 직렬화 단계를 적용했다.
- 현재 graph에는 `<`가 없어 결과 byte에는 치환 대상이 없었지만, closing-script 문자열이 생길 경우 script 경계를 만들지 않는 생성 규칙을 사용했다.

## Revalidation

- graph counts: 130 files, 857 nodes, 1,296 edges, 9 layers, 10 tour steps
- embedded JSON parse: PASS
- offline/external fetch: PASS
- browser combobox keyboard pattern: PASS
- 390px bounded page and internal graph overflow: 기존 PASS 적용
- official dashboard: 환경 제약으로 계속 BLOCKED
