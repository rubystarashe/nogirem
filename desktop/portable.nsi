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
  System::Call 'kernel32::CreateMutexW(p0, i0, w "Global\NogiremPortableCache-${APP_VERSION}") p.r5'
  System::Call 'kernel32::WaitForSingleObject(p r5, i 120000) i.r6'
  StrCmp $6 0 mutex_acquired
  StrCmp $6 128 mutex_acquired
  Abort "포터블 실행 캐시 준비를 기다리는 시간이 초과되었습니다."

  mutex_acquired:
  StrCpy $0 "$PROGRAMFILES64\NogiremPortableRuntime\${APP_VERSION}"
  IfFileExists "$0\portable-ready.marker" cache_ready
  SetOutPath "$0"
  ClearErrors
  File /r "${APP_DIRECTORY}\*"
  IfErrors extraction_failed
  IfFileExists "$0\nogirem.exe" +2
  Goto extraction_failed
  IfFileExists "$0\version.json" +2
  Goto extraction_failed
  ClearErrors
  FileOpen $4 "$0\portable-ready.marker" w
  IfErrors extraction_failed
  FileWrite $4 "${APP_VERSION}$\r$\n"
  IfErrors extraction_failed
  FileClose $4
  Goto cache_ready

  extraction_failed:
  FileClose $4
  Delete "$0\portable-ready.marker"
  RMDir /r "$0"
  System::Call 'kernel32::ReleaseMutex(p r5)'
  System::Call 'kernel32::CloseHandle(p r5)'
  Abort "포터블 실행 파일을 준비하지 못했습니다."

  cache_ready:
  System::Call 'kernel32::ReleaseMutex(p r5)'
  System::Call 'kernel32::CloseHandle(p r5)'

  ${GetParameters} $1
  ExecWait '"$0\nogirem.exe" --portable-source="$EXEPATH" --portable-launcher-pid=$2 $1' $3
  SetErrorLevel $3
SectionEnd
