# Review Request · REV-EXPLAIN-GRAPH

- id: `REV-EXPLAIN-GRAPH`
- run_id: `20260929-project-explain-refresh`
- author: `cursor-coordinator-92251f36`
- recipient: `independent-reviewer`
- created_at: `2026-09-29T20:05:04+09:00`
- reply_to: none
- FEAT/REQ/DEV: `REQ-EXPLAIN-07`, `DEV-EXPLAIN-08`
- feature_impact: none — 제품 동작이 아니라 Rust 코드 지식 그래프와 설명 viewer만 변경
- feature_map: no_change — durable product capability 설명 변경 없음
- stage: implement
- mode: independent
- state: REQUESTED
- target_id: `REV-EXPLAIN-GRAPH-TARGET-01`
- base_commit: `f44454cab129a0dfe3be8f55b8e9e95475ec4f35`
- target_manifest: `target.sha256`

## Requested action

공식 Understand Anything pipeline으로 생성한 `.ua/knowledge-graph.json`, standalone 인터랙티브 `understanding-report.html`, index·manifest·run 기록을 검토한다. 기술 시각화가 문서의 중심인지, graph schema·embedded data·layer·tour가 일치하는지, Rust-only 범위와 offline·접근성·반응형 계약이 유지되는지, 제품 동작 변경을 암시하지 않는지 확인한다.

## Evidence

- graph: 130 files, 857 nodes, 1,296 edges, 9 layers, 10 tour steps
- deterministic validation: issue 0, layer 누락·중복 0, tour dangling 0
- assemble review: 첫 round UTF-8 손상 246건 발견 후 재생성, 재검토 APPROVED
- browser: layer view, `main.rs` 상세 subgraph, Inspector, guided tour 실행 PASS
- mobile: viewport/document/body 390px, graph client 368px, graph internal scroll 900px, Inspector 368px
- lint: pending final IDE check

## Known limits

- 공식 dashboard는 localhost bind `EACCES`와 Node 23/Vite 6 package import 오류로 `BLOCKED`
- 정적 import가 없는 설정·문서·자산 53개는 orphan warning
- 실제 앱 기능·설치·업데이트·driver QA는 이 문서 변경에서 실행하지 않음
