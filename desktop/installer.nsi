Unicode true
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"
Name "마비노기 렘 부스터"
Icon "${APP_DIRECTORY}\icon.ico"
UninstallIcon "${APP_DIRECTORY}\icon.ico"
OutFile "${OUTPUT_FILE}"
InstallDir "$PROGRAMFILES64\Nogirem"
RequestExecutionLevel admin
SetCompressor /SOLID lzma
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\nogirem.exe"
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Korean"

Function EnsureWebView
  SetRegView 32
  ReadRegStr $0 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  ${If} $0 == ""
    ReadRegStr $0 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  ${EndIf}
  ${If} $0 == ""
  ${OrIf} $0 == "0.0.0.0"
    InitPluginsDir
    SetOutPath "$PLUGINSDIR"
    File /oname=MicrosoftEdgeWebview2Setup.exe "${WEBVIEW_BOOTSTRAPPER}"
    DetailPrint "Microsoft WebView2 런타임 설치 중..."
    ExecWait '"$PLUGINSDIR\MicrosoftEdgeWebview2Setup.exe" /silent /install' $0
    ${If} $0 != 0
      MessageBox MB_ICONSTOP "WebView2 런타임 설치에 실패했습니다. 인터넷 연결을 확인하고 다시 설치해 주세요. (코드 $0)"
      Abort
    ${EndIf}
    ReadRegStr $0 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
    ${If} $0 == ""
      ReadRegStr $0 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
    ${EndIf}
    ${If} $0 == ""
    ${OrIf} $0 == "0.0.0.0"
      MessageBox MB_ICONSTOP "WebView2 런타임을 확인하지 못했습니다. 런타임 설치 후 다시 시도해 주세요."
      Abort
    ${EndIf}
  ${EndIf}
  SetRegView 64
FunctionEnd

Section "Install"
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "Windows x64가 필요합니다."
    Abort
  ${EndIf}
  Call EnsureWebView
  InitPluginsDir
  SetOutPath "$PLUGINSDIR"
  File /oname=stop-installed-app.ps1 "${STOP_SCRIPT}"
  ${DisableX64FSRedirection}
  ExecWait '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\stop-installed-app.ps1" -InstallDirectory "$INSTDIR"' $0
  ${EnableX64FSRedirection}
  ${If} $0 != 0
    SetErrorLevel 1
    MessageBox MB_ICONSTOP "실행 중인 앱을 정상 종료한 뒤 다시 설치해 주세요."
    Abort
  ${EndIf}
  !include "legacy-node-cleanup.nsh"
  SetOutPath "$INSTDIR"
  ClearErrors
  File /r "${APP_DIRECTORY}\*"
  IfErrors 0 +3
    SetErrorLevel 1
    Abort "앱 파일을 설치하지 못했습니다"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateShortcut "$DESKTOP\마비노기 렘 부스터.lnk" "$INSTDIR\nogirem.exe" "" "$INSTDIR\icon.ico"
  CreateShortcut "$SMPROGRAMS\마비노기 렘 부스터.lnk" "$INSTDIR\nogirem.exe" "" "$INSTDIR\icon.ico"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\NogiremDioxus" "DisplayName" "마비노기 렘 부스터"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\NogiremDioxus" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\NogiremDioxus" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\NogiremDioxus" "InstallLocation" "$INSTDIR"
  Delete "$APPDATA\마비노기 렘 부스터\instance\installer-close-request"
SectionEnd

Section "Uninstall"
  SetRegView 64
  InitPluginsDir
  SetOutPath "$PLUGINSDIR"
  File /oname=stop-installed-app.ps1 "${STOP_SCRIPT}"
  ${DisableX64FSRedirection}
  ExecWait '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\stop-installed-app.ps1" -InstallDirectory "$INSTDIR" -Uninstall' $0
  ${EnableX64FSRedirection}
  ${If} $0 != 0
    SetErrorLevel 1
    MessageBox MB_ICONSTOP "실행 중인 앱을 정상 종료한 뒤 다시 제거해 주세요."
    Abort
  ${EndIf}
  ; User settings, recordings and original optimization snapshots are retained.
  !include "${UNINSTALL_MANIFEST}"
  Delete "$INSTDIR\Uninstall.exe"
  Delete "$DESKTOP\마비노기 렘 부스터.lnk"
  Delete "$SMPROGRAMS\마비노기 렘 부스터.lnk"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\NogiremDioxus"
  RMDir "$INSTDIR"
SectionEnd
