!macro writeHelperStopRequests
  CreateDirectory "$APPDATA\${PRODUCT_NAME}\affinity"
  FileOpen $0 "$APPDATA\${PRODUCT_NAME}\affinity\control.json" w
  FileWrite $0 `{"command":"stop","requestedAt":0,"reason":"installer-upgrade"}`
  FileClose $0

  CreateDirectory "$APPDATA\${PRODUCT_NAME}\memory"
  FileOpen $0 "$APPDATA\${PRODUCT_NAME}\memory\control.json" w
  FileWrite $0 `{"command":"stop","requestedAt":0,"reason":"installer-upgrade"}`
  FileClose $0
!macroend

!macro customCheckAppRunning
  !insertmacro IS_POWERSHELL_AVAILABLE
  !insertmacro FIND_PROCESS "${APP_EXECUTABLE_FILENAME}" $R0
  ${if} $R0 == 0
    CreateDirectory "$APPDATA\${PRODUCT_NAME}\instance"
    FileOpen $0 "$APPDATA\${PRODUCT_NAME}\instance\installer-close-request" w
    FileWrite $0 "installer-upgrade"
    FileClose $0
    StrCpy $R1 0

    installer_wait_for_app:
      !insertmacro FIND_PROCESS "${APP_EXECUTABLE_FILENAME}" $R0
      ${if} $R0 != 0
        Goto installer_app_closed
      ${endIf}
      IntOp $R1 $R1 + 1
      ${if} $R1 >= 100
        Goto installer_force_close
      ${endIf}
      Sleep 100
      Goto installer_wait_for_app

    installer_force_close:
      !insertmacro writeHelperStopRequests
      Sleep 750
      ExecShellWait "runas" "$SYSDIR\taskkill.exe" `/F /IM "${APP_EXECUTABLE_FILENAME}"` SW_HIDE
      Sleep 500
      !insertmacro FIND_PROCESS "${APP_EXECUTABLE_FILENAME}" $R0
      ${if} $R0 == 0
        MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "실행 중인 ${PRODUCT_NAME}을 종료하지 못했습니다." /SD IDCANCEL IDRETRY installer_force_close
        Quit
      ${endIf}

    installer_app_closed:
      Delete "$APPDATA\${PRODUCT_NAME}\instance\installer-close-request"
  ${endIf}
!macroend
