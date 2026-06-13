; NSIS installer for Nyx (pure-Rust gpui build).
;
; Built in CI with:
;   makensis /DAPPVERSION=<x.y.z> /DSOURCEEXE=<path to nyx.exe> installer\windows\nyx.nsi
;
; Produces:  Nyx_<APPVERSION>_x64-setup.exe
;
; The helper service required for TUN mode is installed by Nyx itself on first
; launch (with an elevation prompt), so this installer only lays down the binary,
; shortcuts and an uninstaller.

Unicode true
SetCompressor /SOLID lzma

!include "MUI2.nsh"
!include "x64.nsh"

!ifndef APPVERSION
  !define APPVERSION "0.0.0"
!endif
!ifndef SOURCEEXE
  !define SOURCEEXE "nyx.exe"
!endif

!define APPNAME      "Nyx"
!define COMPANY      "BX Team"
!define DESCRIPTION  "Mihomo/Clash GUI"
!define UNINSTKEY    "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APPNAME}"
!define ICON         "..\..\assets\brand\icon.ico"

Name "${APPNAME}"
BrandingText "${APPNAME} ${APPVERSION}"
OutFile "Nyx_${APPVERSION}_x64-setup.exe"
InstallDir "$PROGRAMFILES64\${APPNAME}"
InstallDirRegKey HKLM "Software\${APPNAME}" "InstallDir"
RequestExecutionLevel admin

VIProductVersion "${APPVERSION}.0"
VIAddVersionKey "ProductName"     "${APPNAME}"
VIAddVersionKey "CompanyName"     "${COMPANY}"
VIAddVersionKey "FileDescription" "${DESCRIPTION}"
VIAddVersionKey "FileVersion"     "${APPVERSION}"
VIAddVersionKey "ProductVersion"  "${APPVERSION}"
VIAddVersionKey "LegalCopyright"  "GPL-3.0"

!define MUI_ICON   "${ICON}"
!define MUI_UNICON "${ICON}"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\nyx.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch ${APPNAME}"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Function .onInit
  ${IfNot} ${RunningX64}
    MessageBox MB_ICONSTOP "Nyx requires a 64-bit version of Windows."
    Abort
  ${EndIf}
  SetRegView 64
FunctionEnd

Section "Nyx" SecMain
  SectionIn RO
  SetOutPath "$INSTDIR"
  File "/oname=nyx.exe" "${SOURCEEXE}"

  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateDirectory "$SMPROGRAMS\${APPNAME}"
  CreateShortcut  "$SMPROGRAMS\${APPNAME}\${APPNAME}.lnk" "$INSTDIR\nyx.exe"
  CreateShortcut  "$SMPROGRAMS\${APPNAME}\Uninstall ${APPNAME}.lnk" "$INSTDIR\uninstall.exe"
  CreateShortcut  "$DESKTOP\${APPNAME}.lnk" "$INSTDIR\nyx.exe"

  WriteRegStr HKLM "Software\${APPNAME}" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "${UNINSTKEY}" "DisplayName"     "${APPNAME}"
  WriteRegStr HKLM "${UNINSTKEY}" "DisplayVersion"  "${APPVERSION}"
  WriteRegStr HKLM "${UNINSTKEY}" "Publisher"       "${COMPANY}"
  WriteRegStr HKLM "${UNINSTKEY}" "DisplayIcon"     "$INSTDIR\nyx.exe"
  WriteRegStr HKLM "${UNINSTKEY}" "UninstallString" "$INSTDIR\uninstall.exe"
  WriteRegStr HKLM "${UNINSTKEY}" "InstallLocation" "$INSTDIR"
  WriteRegDWORD HKLM "${UNINSTKEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINSTKEY}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  SetRegView 64
  Delete "$INSTDIR\nyx.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir  "$INSTDIR"

  Delete "$SMPROGRAMS\${APPNAME}\${APPNAME}.lnk"
  Delete "$SMPROGRAMS\${APPNAME}\Uninstall ${APPNAME}.lnk"
  RMDir  "$SMPROGRAMS\${APPNAME}"
  Delete "$DESKTOP\${APPNAME}.lnk"

  DeleteRegKey HKLM "${UNINSTKEY}"
  DeleteRegKey HKLM "Software\${APPNAME}"
SectionEnd
