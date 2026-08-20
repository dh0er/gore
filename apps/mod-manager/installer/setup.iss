; gore-mod-manager Windows installer.
; Compiled by CI / build.py:
;   iscc /DAppVersion=<x.y.z> /DSourceDir=<abs path to Release dir> ^
;        /DOutputDir=<abs path for the exe> /DOutputBaseName=<file stem> ^
;        installer\setup.iss
; The wizard shows a directory page, so users pick any install location.
; WinSparkle updates download this same installer and re-run it; Inno
; remembers the previous location and updates in place.

#ifndef AppVersion
  #error "Pass /DAppVersion=x.y.z"
#endif
#ifndef SourceDir
  #error "Pass /DSourceDir=<path to flutter Release dir>"
#endif
#ifndef OutputDir
  #error "Pass /DOutputDir=<path for installer exe>"
#endif
#ifndef OutputBaseName
  #error "Pass /DOutputBaseName=<installer file stem>"
#endif

[Setup]
; Fixed GUID identifies the app across versions for in-place updates.
; Distinct from goresave's and gore-mod's GUIDs so the products install
; side by side.
AppId={{B7E4D2C9-5A18-4F6B-9E3D-1C8A7F4B2D60}
AppName=GORE Mod Manager
AppVersion={#AppVersion}
AppVerName=GORE Mod Manager {#AppVersion}
AppPublisher=Daniel Hoer
DefaultDirName={autopf}\gore-manager
DefaultGroupName=GORE Mod Manager
; Per-user installs work without elevation; the dialog lets the user pick
; all-users (Program Files, admin) or current-user (no UAC prompt).
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir={#OutputDir}
OutputBaseFilename={#OutputBaseName}
VersionInfoCompany=Daniel Hoer
VersionInfoCopyright=Copyright (C) 2026 Daniel Hoer. All rights reserved.
VersionInfoDescription=GORE Mod Manager Setup
VersionInfoOriginalFileName={#OutputBaseName}.exe
VersionInfoProductName=GORE Mod Manager
VersionInfoProductTextVersion={#AppVersion}
VersionInfoProductVersion={#AppVersion}.0
VersionInfoTextVersion={#AppVersion}
VersionInfoVersion={#AppVersion}.0
Compression=lzma
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\gore_manager.exe
WizardStyle=modern
SetupIconFile=..\windows\runner\resources\app_icon.ico
LicenseFile=..\..\..\LICENSE
#ifdef GORE_SIGNED_INSTALLER
; build.py defines this uniquely named /S tool only when GORE_SIGN=1.
; Inno signs both Setup and the embedded uninstaller with the same command.
#ifndef GORE_SIGNED_UNINSTALLER_DIR
  #error "Pass /DGORE_SIGNED_UNINSTALLER_DIR=<temporary path>"
#endif
SignTool=gore_mod_manager_ats_b7e4d2c95a184f6b
SignedUninstaller=yes
SignedUninstallerDir={#GORE_SIGNED_UNINSTALLER_DIR}
#endif

[UninstallDelete]
; Active settings live under the shared gore umbrella
; (%LOCALAPPDATA%\gore\gore-manager\). Remove only gore-manager's own
; subfolder so gore-save / gore-mod / gore-cli data under gore survives.
Type: filesandordirs; Name: "{localappdata}\gore\gore-manager"

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
; LICENSE + third-party attributions live at the repo root, not in SourceDir
; (the Flutter Release dir), so include them explicitly. Paths are relative to
; this .iss (apps\<app>\installer\ -> repo root is ..\..\..).
Source: "..\..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\..\THIRD_PARTY_LICENSES.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\GORE Mod Manager"; Filename: "{app}\gore_manager.exe"
Name: "{autodesktop}\GORE Mod Manager"; Filename: "{app}\gore_manager.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional icons:"

[Run]
Filename: "{app}\gore_manager.exe"; Description: "Launch GORE Mod Manager"; Flags: nowait postinstall skipifsilent
