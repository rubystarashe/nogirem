# 0.4.8 원격 공지 게시

- record_id: `RUN-20261019-NOTICE-PUBLISH-048`
- run_id: `20261019-notice-publish-048`
- checkpoint_id: `CP-NOTICE-PUBLISH-048`
- owner: `cursor-agent-92251f36`
- created_at: `2026-10-19T14:44:00Z`
- request: `현재 NOTICE.md를 커밋·푸시해 GitHub 원격 공지로 반영`
- source_base: `ebd3f058c83726143341ce2c67a5ed6382c37ba4`
- notice_sha256: `90b8803685fdb16e699c60b60d24d300738ff86bb8547a9a9eb481a2dc39cba3`
- status: `PARTIALLY_COMPLETED`
- DEV: `DONE`
- QA: `COMPLETE_FAIL`
- review_mode: `SELF_REVIEW`
- review_verdict: `APPROVED`
- feature_impact: `none — runtime 기능은 바뀌지 않고 앱이 GitHub main에서 읽는 공지 문서 내용만 갱신`
- feature_map: `no_change — 공지 콘텐츠 변경은 현재 canonical product capability 설명을 변경하지 않음`
- architecture_impact: `none — 기존 raw GitHub NOTICE 우선·package fallback 경계를 그대로 사용`
- architecture_contract: `no_change — revision 42의 runtime 경계에 영향 없음`

## 범위와 검증

- included: 사용자 작성 `NOTICE.md` exact content commit, origin/main push, raw GitHub content hash 재검증.
- excluded: 앱 source, package, tag, GitHub Release assets 재변경.
- risk: `LOW — Markdown 공지 내용만 변경하며 runtime·durable data·API·보안 경계는 바뀌지 않는다. 다만 온라인 앱에 즉시 표시되는 외부 콘텐츠이므로 원격 hash를 확인한다.`
- unrelated_preserved: 비추적 `data/`.
- source_review: 기존 원격 공지를 부스트 모드 선택·CPU 개선 안내로 교체하며 credential·개인정보·active content가 없음을 확인.
- notice_commit: `5e93686152bbf87cb63e1207da2f6b389c136244`
- push_result: `PASS — origin/main이 notice_commit을 가리킴`
- github_api_result: `PASS — main의 NOTICE blob 0eea814ab598fb7e9b081d963111fbf3c5fab7fb가 "# 부스트 모드 선택 기능 추가"로 시작`
- raw_commit_url_result: `PASS — notice_commit 고정 URL이 새 공지를 반환`
- raw_branch_url_result: `FAIL — 2026-10-19T14:49Z 현재 앱이 사용하는 main/NOTICE.md URL은 GitHub CDN 캐시로 이전 공지를 반환`
- evidence_limit: `Git 저장소 반영은 완료됐지만 branch raw CDN 캐시가 갱신되기 전까지 온라인 앱에는 이전 공지가 보일 수 있음`
- next_action: GitHub raw CDN 캐시 만료 후 main/NOTICE.md 첫 제목을 재확인한다.
