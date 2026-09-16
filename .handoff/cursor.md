# Cursor AI Handoff

Last Updated: 2026-09-16 21:00

## Current Objective
0.4.0 앱 내 스크롤 품질을 안정화하고, 이후 오버레이 기능을 별도 모듈 방식으로 구현한다.

## Current Status
- 고급 기능에 오버레이 용도와 동작 흐름을 보여주는 인터페이스를 추가했으며 실제 다운로드·캡처 기능은 아직 연결하지 않았다.
- 앱·약관·공지의 중복 커스텀 휠 스크롤을 공통 모듈로 통합했다.
- 연속 휠 입력의 미처리 목표 거리를 화면 높이의 65%로 제한해 과도한 후속 이동을 막았다.
- 보간율을 실제 프레임 경과 시간에 맞추고 100ms 이상 프레임이 지연되면 목표 위치로 즉시 보정한다.
- 전체 Node 테스트 178개, Vite 프로덕션 빌드와 변경 파일 lint가 통과했다.

## Architecture / Important Decisions
- 부드러운 스크롤은 `web/smooth-wheel-scroll.mjs`의 `createSmoothWheelScroller`를 앱·약관·공지에서 공유한다.
- 60Hz 정상 프레임에서는 기존 18% 감속감을 유지하고, 느린 프레임에서는 경과 시간만큼 더 이동한다.
- Ctrl+휠은 앱 스크롤이 가로채지 않으며 line/page 단위 휠 입력은 픽셀로 정규화한다.
- 오버레이는 터보 키처럼 앱과 분리된 선택적 다운로드 모듈로 제공할 예정이다.

## Constraints / Rules
- 코드 주석은 한국어로 작성하고 JavaScript 줄 끝 세미콜론은 사용하지 않는다.
- 오버레이 실제 기능은 요구사항 확정 전 구현하지 않는다.
- Smart App Control 활성만으로 DXVK 설치를 막거나 DLL을 자동 삭제하지 않는다.
- 의미 있는 변경 후 이 파일을 갱신하고 설명 본문이 포함된 semantic commit을 만든다.

## Pending Tasks
1. 실제 앱에서 고해상도 휠과 일반 휠의 스크롤 감각을 확인한다.
2. 오버레이 모듈 배포 자산·무결성 manifest·동의 화면과 독립 다운로드/제거 IPC를 설계한다.
3. 캡처 영역과 표시 위치·크기·투명도 설정 UI를 확정한다.
4. 캡처 helper를 구현하고 안티치트·다중 모니터·DPI 환경을 검증한다.

## Known Issues
- 새 스크롤 보정은 자동 테스트와 빌드만 검증됐으며 실제 장치별 휠 감각 확인이 필요하다.
- 일부 DXVK 버전은 Windows Smart App Control에서 `0xC0E90002`로 차단될 수 있다.
- 오버레이 버튼은 의도적으로 `준비 중` 상태다.

## Key Files
- `web/smooth-wheel-scroll.mjs`: 프레임 지연과 입력 누적을 제한하는 공통 스크롤 제어기
- `web/App.svelte`: 메인 앱과 고급 기능 오버레이 인터페이스
- `web/TermsModal.svelte`: 약관 모달 스크롤 적용
- `web/NoticeModal.svelte`: 공지 모달 스크롤 적용
- `test/smooth-wheel-scroll.test.mjs`: 정상 프레임·누적 제한·지연 보정 회귀 테스트
- `VERSION_HISTORY_DETAIL.md`: 과거 구현과 배포의 상세 기록

## Recent Changes
- 세 화면의 고정 프레임 18% 보간 구현을 공통 시간 기반 스크롤 제어기로 교체했다.
- 386KB까지 누적된 handoff를 현재 목표와 미완료 작업 중심으로 축약했다. 과거 상세 내역은 Git 기록과 `VERSION_HISTORY_DETAIL.md`에 유지한다.
- 0.4.0 오버레이 인터페이스와 별도 모듈 예정 안내를 추가했다.

## Next Recommended Step
개발 앱에서 연속 휠 입력과 렌더러 부하 상황을 재현해 체감 스크롤을 확인한 뒤 필요하면 누적 한도와 보정 임계값만 조정한다.
