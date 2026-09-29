# REV-EXPLAIN-01 Review 01

- id: `REV-EXPLAIN-01-REVIEW-01`
- run_id: `20260929-project-explain-refresh`
- author: `subagent:2b5ebc36-66e3-4832-8d0c-71b10fc193c6`
- recorded_by: coordinator from actual tool output
- created_at: `2026-09-29T18:56:00+09:00`
- reply_to: `ack-01.md`
- stage: implement
- mode: independent
- verdict: `CHANGES_REQUIRED`

## ISSUE-EXPLAIN-01 · MEDIUM · must_fix

- 위치: `understanding-report.html`, “미디어 응답은 Range를 지원하되 한 응답을 4 MiB로 제한한다.”
- 관찰: 연결된 `desktop/backend/src/http.rs`는 outbound HTTPS client다.
- 실제 근거: `desktop/backend/src/blackbox.rs::video_response`
- 수정 조건: 링크를 `blackbox.rs`로 교체한다.

## ISSUE-EXPLAIN-02 · MEDIUM · must_fix

- 위치: `index.html`, `plan.md`, `handoff.md`의 150/144 inventory 숫자
- 관찰: staged target에 전체 inventory나 산출 기준이 없다.
- 수정 조건: 재현 가능한 inventory를 포함하거나 검증되지 않은 숫자를 제거한다.

## ISSUE-EXPLAIN-03 · MEDIUM · must_fix

- 위치: `qa.md`의 browser/responsive PASS
- 관찰: screenshot binary와 접근성 snapshot·실행 로그가 staged target에 없다.
- 수정 조건: 재현 가능한 명령·출력을 `evidence/`에 포함하거나 판정을 미검증으로 낮춘다.

## 정상 확인

- HTML 공통 계약, 외부 의존성 부재, 로컬 링크와 SVG 접근성
- commit·생성 시각 일치
- 주요 Rust GUI/service/worker/helper/update/install/portable 설명
- Electron 구현 범위 제외
- 두 canonical discrepancy의 타당성
