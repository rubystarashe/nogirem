Unicode true
!include "MUI2.nsh"
!include "FileFunc.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"
Name "마비노기 렘 부스터"
Icon "${APP_DIRECTORY}\icon.ico"
UninstallIcon "${APP_DIRECTORY}\icon.ico"
OutFile "${OUTPUT_FILE}"
VIProductVersion "${APP_VERSION}.0"
VIAddVersionKey /LANG=1042 "ProductName" "마비노기 렘 부스터"
VIAddVersionKey /LANG=1042 "FileDescription" "마비노기 렘 부스터 설치 프로그램"
VIAddVersionKey /LANG=1042 "FileVersion" "${APP_VERSION}"
VIAddVersionKey /LANG=1042 "ProductVersion" "${APP_VERSION}"
VIAddVersionKey /LANG=1042 "InternalName" "nogirem-installer"
VIAddVersionKey /LANG=1042 "OriginalFilename" "nogirem-setup.exe"
VIAddVersionKey /LANG=1042 "LegalCopyright" "Copyright (C) 2026 Nogirem contributors"
InstallDir "$PROGRAMFILES64\Nogirem"
RequestExecutionLevel admin
SilentInstall silent
SetCompressor lzma
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
    MessageBox MB_ICONSTOP|MB_YESNO "Microsoft Edge WebView2 Runtime이 필요합니다.$\r$\n공식 다운로드 페이지를 여시겠습니까? 설치 후 이 설치 파일을 다시 실행해 주세요." IDNO webview_missing
    ExecShell "open" "https://developer.microsoft.com/microsoft-edge/webview2/#download"
    webview_missing:
    Abort
  ${EndIf}
  SetRegView 64
FunctionEnd

Section "Install"
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "Windows x64가 필요합니다."
    Abort
  ${EndIf}
  StrCpy $8 "0"
  ${GetParameters} $1
  ClearErrors
  ${GetOptions} "$1" "/NOGIREMUPDATE=" $2
  IfErrors update_flag_done
  IntFmt $3 "%u" $2
  StrCmp $3 $2 0 update_flag_invalid
  IntCmpU $3 0 update_flag_invalid update_flag_invalid update_flag_valid
  update_flag_invalid:
    MessageBox MB_ICONSTOP "업데이트 설치 프로세스 ID가 올바르지 않습니다."
    Abort
  update_flag_valid:
  StrCpy $8 $3
  update_flag_done:
  Call EnsureWebView
  IfFileExists "$INSTDIR\*.*" 0 stop_installed_app_done
  InitPluginsDir
  SetOutPath "$PLUGINSDIR"
  File /oname=installer-cleanup.exe "${CLEANUP_HELPER}"
  System::Call 'kernel32::GetCurrentProcessId() i.r9'
  ${If} $8 == "0"
    nsExec::ExecToStack '"$PLUGINSDIR\installer-cleanup.exe" --install-directory "$INSTDIR" --caller-pid $9'
  ${Else}
    nsExec::ExecToStack '"$PLUGINSDIR\installer-cleanup.exe" --install-directory "$INSTDIR" --caller-pid $9 --updater-pid $8'
  ${EndIf}
  Pop $0
  Pop $1
  DetailPrint "앱 종료 helper 반환 코드: $0"
  ${If} $0 != 0
    SetErrorLevel 1
    MessageBox MB_ICONSTOP "기존 앱을 종료하지 못해 설치를 계속할 수 없습니다. 작업 관리자에서 앱과 앱 업데이트를 종료한 뒤 다시 시도해 주세요."
    Abort
  ${EndIf}
  stop_installed_app_done:
  !include "legacy-node-cleanup.nsh"
  !include "retired-assets-cleanup.nsh"
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
  ${GetParameters} $1
  ClearErrors
  ${GetOptions} "$1" "/S" $2
  IfErrors launch_app
  Goto install_done

  launch_app:
  Exec '"$INSTDIR\nogirem.exe"'

  install_done:
SectionEnd

Section "Uninstall"
  SetRegView 64
  InitPluginsDir
  SetOutPath "$PLUGINSDIR"
  File /oname=installer-cleanup.exe "${CLEANUP_HELPER}"
  System::Call 'kernel32::GetCurrentProcessId() i.r9'
  nsExec::ExecToStack '"$PLUGINSDIR\installer-cleanup.exe" --install-directory "$INSTDIR" --caller-pid $9 --uninstall'
  Pop $0
  Pop $1
  DetailPrint "앱 종료 helper 반환 코드: $0"
  ${If} $0 != 0
    SetErrorLevel 1
    MessageBox MB_ICONSTOP "앱을 종료하지 못해 제거를 계속할 수 없습니다. 작업 관리자에서 앱과 앱 업데이트를 종료한 뒤 다시 시도해 주세요."
    Abort
  ${EndIf}
  ; 사용자 설정, 녹화 파일, 최적화 원본 스냅샷은 유지한다.
  !include "${UNINSTALL_MANIFEST}"
  !include "retired-assets-cleanup.nsh"
  Delete "$INSTDIR\Uninstall.exe"
  Delete "$DESKTOP\마비노기 렘 부스터.lnk"
  Delete "$SMPROGRAMS\마비노기 렘 부스터.lnk"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\NogiremDioxus"
  RMDir "$INSTDIR"
SectionEnd
