# 업데이트 전환 핫픽스 계획

- DEV-HOTFIX-01 (`VERIFIED`): 0.4.0 다운로드 정지와 제거 차단 경로 진단
  - links: REQ-HOTFIX-04, REQ-HOTFIX-05
  - files: `desktop/backend/src/updater.rs`, `desktop/backend/src/service.rs`, `scripts/stop-installed-app.ps1`
- DEV-HOTFIX-02 (`VERIFIED`): Electron 0.3.19 전환 상태·재시도 연결
  - links: REQ-HOTFIX-01, REQ-HOTFIX-02
  - files: `scripts/prepare-electron-bridge.mjs`, `desktop/src/main.rs`, `desktop/backend/src/update_install.rs`
- DEV-HOTFIX-03 (`VERIFIED`): 릴리스 조립·라우팅 계약을 0.3.19로 변경
  - links: REQ-HOTFIX-01, REQ-HOTFIX-03
  - files: `scripts/prepare-release.mjs`, `scripts/test-release-routing.mjs`, 배포 문서
- DEV-HOTFIX-04 (`VERIFIED`): 테스트·실서명 패키징·독립 리뷰
  - links: 전체 요구사항
- DEV-HOTFIX-05 (`VERIFIED`): v0.4.1 자산 순차 대체와 공개 검증
  - links: REQ-HOTFIX-03
- DEV-HOTFIX-06 (`VERIFIED`): 실제 0.4.0 사용자 패닉에 따른 blocking 다운로드·오류 재시도 수정과 0.4.1 재대체
  - links: REQ-HOTFIX-03, REQ-HOTFIX-06
  - files: `desktop/backend/src/updater.rs`, `desktop/backend/src/update_install.rs`, `desktop/src/ui.rs`, `desktop/src/smoke.rs`, `web/update-preview.css`, `src/migration-attempt.mjs`, `scripts/prepare-electron-bridge.mjs`, 관련 테스트·배포 문서
- DEV-HOTFIX-07 (`VERIFIED`): 포터블 동일 버전 캐시·복구 권한 경계·파일 교체 경쟁·중단 캐시 정리 보강
  - links: REQ-HOTFIX-08, REQ-HOTFIX-09, REQ-HOTFIX-10
  - files: `desktop/portable.nsi`, `scripts/package-dioxus.mjs`, `desktop/backend/src/update_install.rs`, `test/portable-release.test.mjs`
- DEV-HOTFIX-08 (`VERIFIED`): 일반 업데이트 작업 상태 누락과 상위 디렉터리 junction 교체 경쟁 수정
  - links: REQ-HOTFIX-09
  - files: `desktop/backend/src/update_install.rs`
- DEV-HOTFIX-09 (`VERIFIED`): 제거기 자기 종료·예약 작업 열거 실패 수정과 UI 이전 bootstrap 진단 추가
  - links: REQ-HOTFIX-04, REQ-HOTFIX-11, REQ-HOTFIX-12
  - files: `desktop/installer.nsi`, `scripts/stop-installed-app.ps1`, `desktop/src/main.rs`, `test/installer.test.mjs`

## 검증 시나리오

- 0.3.17 이하가 0.3.19 bridge를 최신 Electron 업데이트로 선택한다.
- 이미 실패한 0.3.18과 0.4.0은 각각 0.3.19와 수정된 0.4.1 설치본으로 수동 복구한다.
- 0.3.19에서 Rust 전환 확인·다운로드 진행·오류·재시도 상태를 전달한다.
- 전환 상태 파일은 UUID 시도 식별자를 검증하고 다른 시도의 상태를 무시한다.
- 정상 종료 요청이 성공하면 강제 종료를 사용하지 않는다.
- 정상 종료 제한 시간을 넘기면 설치 폴더 내부 프로세스만 종료한다.
- 앱 종료 시 다운로드 취소 플래그를 설정한다.
- 공개 서명 매니페스트와 설치 자산을 Tokio runtime 없이 실제 다운로드하고 해시를 검증한다.
