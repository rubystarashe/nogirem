# 0.4.2 진단 로깅 대치 배포

- run_id: `20261001-repackage-042`
- checkpoint_id: `CP-REPACKAGE-042`
- owner: `Cursor Agent`
- created_at: `2026-10-01T14:52:00Z`
- repository: `rubystarashe/nogirem`
- branch: `main`
- source_base: `e537e0bbd48f0ebe506a62939173d1f76f4a8815`
- release: `https://github.com/rubystarashe/nogirem/releases/tag/v0.4.2`
- roles: `Cursor Agent = Coordinator + DEV + QA + Documentation maintainer`, reviewer는 패키지 생성 후 별도 agent에 요청
- feature_impact: `FEAT-NOGIREM-UPDATE-LIFECYCLE — 동일 0.4.2 설치형·포터블 payload와 signed manifest를 진단 로깅 포함 build로 대치`
- feature_map: `no_change — 기존 자동 업데이트·수동 다운로드 동작과 수용 기준은 유지`
- architecture_impact: `none: 기존 패키징, Ed25519 manifest, 설치형·포터블 및 Electron migration 경계를 그대로 사용`
- architecture_contract: `no_change — 기존 계약이 이번 대치에도 정확함`

## 요청과 권한

- 요청: affinity 전환과 Windows WHEA·GPU·LiveKernelEvent 로깅이 포함된 Rust/Dioxus 0.4.2를 패키징하고 기존 GitHub v0.4.2 자산을 대치한다.
- 허용: 로컬 패키징, 서명, v0.4.2 동일 이름 자산 업로드·대치, 공개 다운로드 검증, 로컬 commit.
- 제외: tag rewrite, main push, 새 버전 생성, Electron migration payload 변경, 배포 외 운영 변경.

## 위험과 제한

- risk: `HIGH — 이미 공개된 자동 업데이트 payload와 manifest를 동일 버전으로 교체`
- 기존 0.4.2는 같은 버전을 더 새 버전으로 판단하지 않으므로 자동으로 대치본을 다시 받지 않는다. 신규 설치, 수동 재설치, 0.4.1 이하의 0.4.2 업데이트에 적용된다.
- release tag는 기존 source `86ca9261ab763e57477c7655d3f303b256d29bd8`을 유지하며 rewrite하지 않는다. 배포 payload source는 별도 exact commit으로 기록한다.
- Electron 0.3.19 migration installer, blockmap, `latest.yml`은 현재 공개 자산을 byte-identical로 보존한다.

## 계획과 상태

1. `IN_PROGRESS` — release 기준선, 서명키 공개키 일치, package source 확인
2. `NOT_STARTED` — signed 설치형·포터블과 manifest 생성
3. `NOT_STARTED` — package contract, digest, signature, alias identity와 독립 review
4. `NOT_STARTED` — GitHub v0.4.2 자산 대치, labels 복원, CDN 실다운로드 검증
5. `NOT_STARTED` — handoff, QA, final disposition과 로컬 commit

## 중단 조건

- 서명키 공개키 불일치, package 검증 실패, 독립 review mandatory finding, manifest/payload digest 불일치, Electron feed 변화, 공개 CDN 불일치 시 업로드 또는 완료를 중단한다.

## 현재 상태

- DEV: `IN_PROGRESS`
- QA: `PLANNED`
- review: `NOT_REVIEWED`
- release: `NOT_STARTED`
- overall: `PARTIALLY_COMPLETED`
