# 업데이트 전환 핫픽스 QA

## QA-HOTFIX-01

- links: REQ-HOTFIX-01, REQ-HOTFIX-02
- environment: Windows 10, 생성된 Electron 0.3.19 소스·패키지
- expected: 0.3.18 이하가 0.3.19를 선택하고 전환 상태·오류·재시도를 표시
- actual: 라우팅 계약에서 v0.4.0·v0.4.1 모두 0.3.19 설치본을 선택했다. 생성 소스의 Node 문법, 오류 패널, 재시도·숨기기 버튼, 60초 무응답 제한을 확인했다.
- status: PASS

## QA-HOTFIX-02

- links: REQ-HOTFIX-04
- environment: 서로 다른 임시 폴더에서 실행한 동일 이름 `nogirem.exe` 테스트 프로세스 2개
- expected: 대상 설치 폴더 프로세스만 종료하고 다른 경로의 동명 프로세스는 유지
- actual: 정상 종료 요청 30초 뒤 대상 프로세스만 제한 종료됐고 다른 경로 프로세스는 계속 실행됐다.
- status: PASS

## QA-HOTFIX-03

- links: REQ-HOTFIX-03, REQ-HOTFIX-05
- environment: Rust·Node 로컬 빌드
- expected: 코드·계약 테스트와 실서명 0.4.1 패키징 성공
- actual:
  - 전체 Node 테스트 187개 통과
  - Rust backend 전체 테스트 통과
  - Rust desktop 8개 중 관련 7개 통과, 기존 Markdown fixture의 `342.0MiB` 기대값 1개 실패
  - IDE 린트 오류 없음
  - 0.4.1 설치형·포터블 실서명 패키징과 매니페스트 검증 통과
- status: PASS

## 남은 실제 QA

- 0.3.17·0.3.18 → 0.3.19 → Rust 0.4.1 실제 설치 전환
- 실제 UAC 승인·거부와 오류 후 재시도
- 실제 0.4.0 고착 환경에서 대체 0.4.1 수동 설치 후 제거

## QA-HOTFIX-04

- links: REQ-HOTFIX-01, REQ-HOTFIX-03
- environment: GitHub Latest Release `v0.4.1`
- expected: 대체 자산 해시와 서명, Electron 라우팅이 최종 로컬 산출물과 일치
- actual: 대체한 자산의 GitHub SHA-256 digest가 로컬과 일치했다. 공개 `update.json`·`portable-update.json` 서명과 0.4.1 URL, `latest.yml`의 0.3.19 라우팅을 확인했다.
- status: PASS

## QA-HOTFIX-05

- links: REQ-HOTFIX-06
- environment: 실제 0.4.0 사용자 진단 ZIP, reqwest 0.12.28, 공개 GitHub `v0.4.1` 서명 자산
- expected: blocking 다운로드가 Tokio runtime 없이 완료되고 오류 시 원인과 재시도를 표시
- actual: 사용자 로그에서 다운로드 시작마다 `there is no reactor running` 패닉을 확인했다. async client 변환을 제거한 뒤 로컬 HTTP 기본 회귀 검사와 공개 매니페스트·설치 자산 다운로드 및 SHA-256 검증이 통과했다. Rust 오류 표시→숨김→배지 재시도 UI 스모크도 통과했다. 0.3.19 생성 소스는 실패 화면 숨김·업데이트 버튼 재표시, helper stderr 진단 기록, timeout 시 이전 helper 종료·attempt 무효화를 포함하며 Svelte 빌드를 통과했다. Rust named mutex의 중복 전환 차단 테스트와 독립 리뷰 2건도 승인됐다.
- status: PASS

## QA-HOTFIX-06

- links: REQ-HOTFIX-03, REQ-HOTFIX-06
- environment: GitHub Latest Release `v0.4.1`
- expected: 승인 산출물 7개가 공개되고 0.4.0 클라이언트 방식의 blocking 다운로드·서명 검증이 완료
- actual: 공개 7개 자산의 GitHub SHA-256 digest가 최종 bundle과 모두 일치했다. 공개 `update.json`을 조회해 새 설치 자산을 실제 다운로드하고 서명 매니페스트·크기·SHA-256 검증을 완료했다.
- status: PASS

## QA-HOTFIX-07

- links: REQ-HOTFIX-01, REQ-HOTFIX-02, REQ-HOTFIX-07
- environment: GitHub Latest Release `v0.4.1`, Electron 0.3.16·0.3.18 provider 계약
- expected: 0.3.17 이하는 0.3.19를 자동 선택하고, 이미 실패한 0.3.18은 공개 0.3.19 설치본으로 수동 복구 가능
- actual: provider 계약의 v0.4.0·v0.4.1 태그와 0.3.16·0.3.18 버전 4개 조합은 모두 0.3.19 URL을 선택했다. 다만 배포된 0.3.18 앱은 Electron 확인 경로를 Rust 전환으로 가로채므로 자동 복구할 수 없어 수동 설치가 필요하다. 공개 0.3.19 설치본·blockmap·latest.yml SHA-256이 최종 bundle과 일치했고 실패 helper가 포함된 0.3.18 자산은 제거했다.
- status: PASS

## QA-HOTFIX-08

- links: REQ-HOTFIX-08, REQ-HOTFIX-09, REQ-HOTFIX-10
- environment: Windows 10, Rust 0.4.1 backend, NSIS 3.0.4.1, 실서명 실제 패키징
- expected: 동일 버전의 다른 빌드는 다른 캐시 digest를 사용하고, 포터블 교체·복구가 보호된 작업과 파일만 수락하며, 복구 가능성이 없는 오래된 캐시를 정리
- actual:
  - 포터블 계약 테스트 5개와 전체 Node 테스트 192개 통과
  - Rust backend 테스트 53개 통과, 공개 자산 실다운로드 검사 1개는 기본 실행에서 제외
  - NSIS 설치형·포터블 실제 실서명 패키징과 매니페스트 파일 크기·SHA-256 검증 통과, 최종 `portableCacheId=d4d7aff2a8674936289b5c13d76cd519927f228282a9ecb70ad658091863b38c`
  - 직전 0.4.1 빌드 digest `859bd7014968b5022c6d7e516c78bf41418966bcbfaa6fe921d66cdc53cf0843`와 달라 동일 버전의 다른 내부 런타임 감지 확인
  - 독립 리뷰 1차에서 동기 롤백 marker 잔존을 수정하고 2차에서 `APPROVED`
  - GitHub v0.4.1의 Rust 자산 4개를 대체하고 원격 digest와 로컬 SHA-256 일치 확인
  - 공개 설치형 blocking 다운로드와 공개 포터블 실다운로드의 크기·SHA-256 확인
  - 전체 desktop 테스트는 기존 Markdown fixture의 `342.0MiB` 기대값 1건만 실패하고 나머지 7건 통과
- status: PASS
