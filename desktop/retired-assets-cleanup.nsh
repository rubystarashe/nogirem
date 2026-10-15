; 이전 설치형이 배포했지만 현재 제품에서는 제거된 앱 소유 파일만 정리한다.
; 설치 폴더 전체나 알 수 없는 사용자 파일은 재귀적으로 삭제하지 않는다.
Delete "$INSTDIR\TURBO_KEY_TERMS.md"
Delete "$INSTDIR\overlay-settings.html"
Delete "$INSTDIR\debuff-region-overlay.html"
Delete "$INSTDIR\debuff-alert-overlay.html"
Delete "$INSTDIR\overlay-configuration.html"
Delete "$INSTDIR\debuff.csv"
Delete "$INSTDIR\service\overlay-settings-preload.js"
Delete "$INSTDIR\service\debuff-region-overlay-preload.js"
Delete "$INSTDIR\service\debuff-alert-overlay-preload.js"
Delete "$INSTDIR\service\overlay-configuration-preload.js"
Delete "$INSTDIR\public\main3.jpg"
Delete "$INSTDIR\public\doc_minimize\introduce.png"

; 현재 파일 제거 뒤 비어 있는 이전 하위 폴더만 제거한다.
RMDir "$INSTDIR\service"
RMDir "$INSTDIR\public\doc_minimize"
RMDir "$INSTDIR\public"
