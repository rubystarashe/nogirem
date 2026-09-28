# 포터블 배포와 업데이트

## 배포 형식

- 설치형: `nogirem-dioxus-setup-<version>.exe`
- 포터블: `nogirem-dioxus-portable-<version>.exe`
- 사용자가 보관하고 이동하는 포터블 배포물은 EXE 한 개다.
- launcher는 첫 실행에 내부 앱을 관리자 전용 Program Files 버전 캐시에 풀고 이후 실행에서 재사용한다.
- 배포 EXE 자체는 변경하지 않으므로 다른 위치로 복사하거나 이동해도 단독으로 실행할 수 있다.
- 내부 앱은 `portable.marker`와 launcher가 전달한 원본 경로·PID로 포터블 모드를 검증한다.

## 데이터와 런타임

- 설치형과 포터블은 `%APPDATA%\마비노기 렘 부스터` 설정을 공유한다.
- WebView2 프로필은 `%LOCALAPPDATA%\Nogirem`, 업데이트 임시 파일은 보호된 `%LOCALAPPDATA%\NogiremUpdater`를 사용한다.
- 녹화 청크는 기존 블랙박스 드라이브 선택 설정을 따른다.
- 포터블에도 관리자 권한 요청이 적용된다.
- WebView2 런타임은 포터블 EXE에 포함하지 않으므로 시스템에 설치되어 있어야 한다.

## 업데이트

- 설치형은 서명된 `update.json`과 NSIS 설치 파일을 사용한다.
- 포터블은 서명된 `portable-update.json`과 새 단일 EXE를 사용한다.
- 두 매니페스트를 분리해 기존 설치형 클라이언트의 스키마 호환성을 유지한다.
- 포터블 업데이트 helper는 GUI·서비스·launcher 종료를 기다린 뒤 원본 포터블 EXE를 원자 교체한다.
- 업데이트 다운로드와 백업은 reparse 검증 및 High 무결성이 적용된 전용 캐시를 사용한다.
- 새 앱이 UI·백엔드 준비 상태를 알리지 못하거나 업데이트가 중단되면 백업 EXE의 SHA-256을 검증해 복원한다.
- 사용자 설정과 녹화 파일은 교체 대상이 아니다.

## 빌드

- 배포용: `npm run package:win`
- 로컬 산출물 확인용: `npm run package:win:unsigned`
- 무서명 빌드는 업데이트 매니페스트를 만들지 않으며 배포할 수 없다.
