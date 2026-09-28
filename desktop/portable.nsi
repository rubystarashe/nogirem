Unicode true
!include "FileFunc.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"
Name "마비노기 렘 부스터 포터블"
Icon "${APP_DIRECTORY}\icon.ico"
OutFile "${OUTPUT_FILE}"
RequestExecutionLevel admin
SilentInstall silent
SetCompressor /SOLID lzma

Section
  ${IfNot} ${RunningX64}
    Abort "Windows x64가 필요합니다."
  ${EndIf}
  System::Call 'kernel32::GetCurrentProcessId() i.r2'
  StrCpy $0 "$PROGRAMFILES64\NogiremPortableRuntime\${APP_VERSION}"
  IfFileExists "$0\portable-ready.marker" launch
  SetOutPath "$0"
  File /r "${APP_DIRECTORY}\*"
  FileOpen $4 "$0\portable-ready.marker" w
  FileWrite $4 "${APP_VERSION}$\r$\n"
  FileClose $4

  launch:
  ${GetParameters} $1
  ExecWait '"$0\nogirem.exe" --portable-source="$EXEPATH" --portable-launcher-pid=$2 $1' $3
  SetErrorLevel $3
SectionEnd
