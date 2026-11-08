; -*- coding: utf-8 -*-
Unicode true
!include "MUI2.nsh"
!include "config.nsh"
!include "x64.nsh"
!include "WinVer.nsh"
!include "FileFunc.nsh"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_ID}"
Name "${APP_NAME}"
OutFile "${OUTPUT_FILE}"
InstallDir "$LOCALAPPDATA\Programs\${APP_ID}"
InstallDirRegKey HKCU "${UNINSTALL_KEY}" "InstallLocation"
RequestExecutionLevel user
ManifestDPIAware true
SetCompressor /SOLID lzma
VIProductVersion "${APP_VERSION}.0"
VIAddVersionKey /LANG=2052 "ProductName" "${APP_NAME}"
VIAddVersionKey /LANG=2052 "FileDescription" "${APP_NAME}安装程序"
VIAddVersionKey /LANG=2052 "FileVersion" "${APP_VERSION}"
VIAddVersionKey /LANG=2052 "LegalCopyright" "NEXORA"
!define MUI_ICON "${PROJECT_DIR}\assets\nexora-isle.ico"
!define MUI_UNICON "${PROJECT_DIR}\assets\nexora-isle.ico"
!define MUI_ABORTWARNING
Var DesktopChoice
Var AutoStartChoice
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_TITLE "NEXORA ISLE · 岛屿 已安装"
!define MUI_FINISHPAGE_TEXT "安装完成，可以从桌面或开始菜单打开。"
!define MUI_FINISHPAGE_RUN "$INSTDIR\nexora-isle.exe"
!define MUI_FINISHPAGE_RUN_TEXT "打开 NEXORA ISLE"
!insertmacro MUI_PAGE_FINISH
!define MUI_UNCONFIRMPAGE_TEXT_TOP "将卸载 NEXORA ISLE · 岛屿。设置、文件口袋和连接记录会保留。"
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"
BrandingText "NEXORA ISLE · 岛屿"
SetFont "Microsoft YaHei UI" 9

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP "NEXORA ISLE 需要 Windows 10 或 Windows 11 的 64 位系统。" /SD IDOK
    SetErrorLevel 1
    Quit
  ${EndIf}
  ${IfNot} ${AtLeastWin10}
    MessageBox MB_OK|MB_ICONSTOP "NEXORA ISLE 需要 Windows 10 或 Windows 11。" /SD IDOK
    SetErrorLevel 1
    Quit
  ${EndIf}
  SetShellVarContext current
  SetRegView 64
  StrCpy $DesktopChoice ${INSTALL_DESKTOP_SHORTCUT}
  StrCpy $AutoStartChoice ${ENABLE_AUTOSTART}
  ReadRegStr $0 HKCU "${UNINSTALL_KEY}" "InstallLocation"
  ${If} $0 != ""
    IfFileExists "$DESKTOP\${APP_NAME}.lnk" +2 0
      StrCpy $DesktopChoice 0
  ${EndIf}
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APP_ID}"
  ${If} $0 != ""
    StrCpy $AutoStartChoice 1
  ${EndIf}
  ; 静默安装使用同一组选项；/D 路径由 NSIS 自身处理。
  ${GetParameters} $0
  ${GetOptions} $0 "/DESKTOP=" $1
  ${If} $1 == "0"
  ${OrIf} $1 == "1"
    StrCpy $DesktopChoice $1
  ${EndIf}
  ${GetOptions} $0 "/AUTOSTART=" $1
  ${If} $1 == "0"
  ${OrIf} $1 == "1"
    StrCpy $AutoStartChoice $1
  ${EndIf}
FunctionEnd

; 程序运行时请用户正常退出，不强制结束进程或写入正在使用的程序。
!macro ClosedCheck Prefix
Function ${Prefix}CheckClosed
  retry:
  System::Call 'kernel32::OpenMutexW(i 0x100000, i 0, w "Local\${APP_ID}.SingleInstance") p .r0'
  ${If} $0 != 0
    System::Call 'kernel32::CloseHandle(p r0)'
    MessageBox MB_RETRYCANCEL|MB_ICONINFORMATION "请先从系统托盘退出${APP_NAME}，然后点击重试。" /SD IDCANCEL IDRETRY retry
    SetErrorLevel 2
    Abort
  ${EndIf}
FunctionEnd
!macroend
!insertmacro ClosedCheck ""
!insertmacro ClosedCheck "un."

Section "安装"
  Call CheckClosed
  SetOutPath "$INSTDIR"
  SetOverwrite on
  File /oname=nexora-isle.exe "${SOURCE_EXE}"
  File "${PROJECT_DIR}\LICENSE"
  File "${PROJECT_DIR}\THIRD_PARTY_NOTICES.txt"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  ${If} $DesktopChoice == 1
    CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\nexora-isle.exe"
  ${Else}
    Delete "$DESKTOP\${APP_NAME}.lnk"
  ${EndIf}
  ${If} $AutoStartChoice == 1
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APP_ID}" '"$INSTDIR\nexora-isle.exe"'
  ${Else}
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APP_ID}"
  ${EndIf}
  ; 从早期版本升级到 ISLE 后，避免旧启动项和快捷方式继续残留。
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "NexoraHalo"
  Delete "$DESKTOP\NEXORA HALO.lnk"
  Delete "$SMPROGRAMS\NEXORA HALO.lnk"
  Delete "$SMPROGRAMS\NEXORA HALO 设置.lnk"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}.lnk" "$INSTDIR\nexora-isle.exe"
  CreateShortcut "$SMPROGRAMS\${APP_NAME} 设置.lnk" "$INSTDIR\nexora-isle.exe" "--settings"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "NEXORA ISLE"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\nexora-isle.exe,0"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
  Delete "$INSTDIR\uninstall.ps1"
SectionEnd

Function un.onInit
  SetShellVarContext current
  SetRegView 64
FunctionEnd

Section "Uninstall"
  Call un.CheckClosed
  Delete "$INSTDIR\nexora-isle.exe"
  Delete "$INSTDIR\nexora-isle-previous.exe"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\THIRD_PARTY_NOTICES.txt"
  Delete "$INSTDIR\uninstall.ps1"
  Delete "$INSTDIR\Uninstall.exe"
  Delete "$DESKTOP\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}设置.lnk"
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APP_ID}"
  ${If} $0 == '"$INSTDIR\nexora-isle.exe"'
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${APP_ID}"
  ${EndIf}
  DeleteRegKey HKCU "${UNINSTALL_KEY}"
  ; 只移除空的安装目录，用户数据和用户自行放入的文件都保留。
  RMDir "$INSTDIR"
SectionEnd
