# 업데이트 전환 핫픽스 QA

## QA-HOTFIX-01

- links: REQ-HOTFIX-01, REQ-HOTFIX-02
- environment: Windows 10, 생성된 Electron 0.3.18 소스·패키지
- expected: 0.3.17이 0.3.18을 선택하고 전환 상태·오류·재시도를 표시
- actual: 라우팅 계약에서 v0.4.0·v0.4.1 모두 0.3.18 설치본을 선택했다. 생성 소스의 Node 문법, 오류 패널, 재시도 버튼, 60초 무응답 제한을 확인했다.
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

- 0.3.17 → 0.3.18 → Rust 0.4.1 실제 설치 전환
- 실제 UAC 승인·거부와 오류 후 재시도
- 실제 0.4.0 고착 환경에서 대체 0.4.1 수동 설치 후 제거
