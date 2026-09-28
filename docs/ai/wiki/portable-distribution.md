# 포터블 배포와 업데이트

## 배포 형식

- 설치형: `nogirem-dioxus-setup-<version>.exe`
- 포터블: `nogirem-dioxus-portable-<version>.zip`
- 포터블 여부는 실행 파일 옆 `portable.marker`로 판단한다.
- 포터블 ZIP은 `portable-manifest.json`에 모든 소유 파일과 SHA-256을 기록한다.

## 데이터와 런타임

- 설치형과 포터블은 `%APPDATA%\마비노기 렘 부스터` 설정을 공유한다.
- WebView2 프로필은 `%LOCALAPPDATA%\Nogirem`, 업데이트 임시 파일은 보호된 `%LOCALAPPDATA%\NogiremUpdater`를 사용한다.
- 녹화 청크는 기존 블랙박스 드라이브 선택 설정을 따른다.
- 포터블에도 관리자 권한 요청이 적용된다.
- WebView2 런타임은 ZIP에 포함하지 않으므로 시스템에 설치되어 있어야 한다.

## 업데이트

- 설치형은 서명된 `update.json`과 NSIS 설치 파일을 사용한다.
- 포터블은 서명된 `portable-update.json`과 ZIP을 사용한다.
- 두 매니페스트를 분리해 기존 설치형 클라이언트의 스키마 호환성을 유지한다.
- 포터블 업데이트 helper는 앱 종료를 기다린 뒤 매니페스트 소유 파일만 교체한다.
- ZIP 내부 소유 파일 매니페스트도 Ed25519 서명을 검증하며 업데이트 캐시는 High 무결성으로 보호한다.
- 새 앱이 UI·백엔드 준비 상태를 알리지 못하면 이전 파일을 복원하고 재실행한다.
- 사용자 설정과 녹화 파일은 교체 대상이 아니다.

## 빌드

- 배포용: `npm run package:win`
- 로컬 산출물 확인용: `npm run package:win:unsigned`
- 무서명 빌드는 업데이트 매니페스트를 만들지 않으며 배포할 수 없다.
- 생성 ZIP 검증: `node scripts/verify-portable-package.mjs <zip> <version>`
