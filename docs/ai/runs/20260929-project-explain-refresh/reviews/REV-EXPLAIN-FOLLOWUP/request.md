# REV-EXPLAIN-FOLLOWUP Request

- id: `REV-EXPLAIN-FOLLOWUP`
- run_id: `20260929-project-explain-refresh`
- author: coordinator
- recipient: `subagent:2b5ebc36-66e3-4832-8d0c-71b10fc193c6`
- created_at: `2026-09-29T19:22:00+09:00`
- reply_to: 없음
- stage: implement
- mode: independent
- REQ: REQ-EXPLAIN-02~04
- target_id: `target-01.sha256`
- base_commit: `b2f16d963adc62060e0050023ac26742da178924`
- target_commit: uncommitted

## 요청

완료된 Rust 기능 분석에서 확인된 복구 한계를 기존 설명 보고서에 후속 반영했다.

- CPU affinity reset과 비정상 종료 후 원래 custom mask 복구 범위
- 패스트핑과 구분되는 TCP autotuning 원본 미보존
- NVIDIA/Radeon/VSync 설정의 원본 snapshot·restore 부재
- 설치형 rollback이 새로 추가된 파일 제거를 보장하지 않는 범위
- affinity worker의 독립 수명과 앱/Dioxus 버전 구분
- 실제 390px CSS viewport에서 발견한 ELI5 code link overflow 수정

`target-01.sha256`의 다섯 파일을 고정 target으로 삼아 source 주장 정확성, 과장·누락, HTML 가독성, manifest 일치, QA 증거의 정합성을 검토한다. reviewed code와 문서는 수정하지 말고 `APPROVED`, `CHANGES_REQUIRED`, `BLOCKED` 중 하나와 stable ISSUE ID를 반환한다.

## 검증 근거

- 정적 HTML·링크·offline·sourceFiles 45개·SVG 접근성: PASS
- 브라우저 접근성 snapshot과 390×844 CSS viewport: PASS
- 실제 앱·GPU·네트워크·설치 동작: 이번 후속 문서 변경에서 NOT_RUN
