# REV-EXPLAIN-FOLLOWUP Review 01

- id: `REV-EXPLAIN-FOLLOWUP-REVIEW-01`
- run_id: `20260929-project-explain-refresh`
- author: `subagent:2b5ebc36-66e3-4832-8d0c-71b10fc193c6`
- recorded_by: coordinator from actual tool output
- created_at: `2026-09-29T19:24:00+09:00`
- reply_to: `ack-01.md`
- stage: implement
- mode: independent
- target_id: `target-01.sha256`
- verdict: `CHANGES_REQUIRED`

## ISSUE-EXPLAIN-FOLLOWUP-01

- severity: MEDIUM
- must_fix: true
- location: `docs/explain/20260929-1835-rust-full/understanding-report.html:43`
- observed evidence: `desktop/Cargo.toml`의 `version = "0.7.3"`은 호환 범위이고 `desktop/Cargo.lock` 및 `cargo metadata --locked`의 실제 resolution은 Dioxus `0.7.10`이다.
- requested correction: 앱 버전 `0.4.1`, 선언 요구값 `^0.7.3`, locked 버전 `0.7.10`을 구분하고 `Cargo.lock`을 근거와 manifest에 추가한다.
- acceptance: Dioxus 실제 버전을 `0.7.3`으로 단정하지 않고 세 버전의 의미를 명확히 구분한다.

## 그 외 결과

- target hash 다섯 개 일치
- affinity reset/full mask와 시작 시 runtime state 삭제 설명 일치
- TCP autotuning 원본 미보존 설명 일치
- GPU snapshot/restore 부재와 Radeon global scope 설명 일치
- 설치형 rollback 신규 파일 미제거 설명 일치
- affinity worker parent PID 비의존 설명 일치
- ELI5 wrapping CSS와 QA evidence 정합성 일치
- 실제 앱·GPU·네트워크·설치·rollback 동작은 미실행
