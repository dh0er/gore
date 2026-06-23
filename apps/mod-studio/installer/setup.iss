; gore-mod Windows installer.
; Compiled by CI / build.py:
;   iscc /DAppVersion=<x.y.z> /DSourceDir=<abs path to Release dir> ^
;        /DOutputDir=<abs path for the exe> installer\setup.iss
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

[Setup]
; Fixed GUID identifies the app across versions for in-place updates.
; Distinct from goresave's GUID so the two products install side by side.
AppId={{F2B9C4A7-8D63-4F1E-A0C5-7E3B6D2A91F4}
AppName=gore-mod
AppVersion={#AppVersion}
AppVerName=gore-mod {#AppVersion}
AppPublisher=dh0er
DefaultDirName={autopf}\gore-mod
DefaultGroupName=gore-mod
; Per-user installs work without elevation; the dialog lets the user pick
; all-users (Program Files, admin) or current-user (no UAC prompt).
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir={#OutputDir}
OutputBaseFilename=GoreModSetup-{#AppVersion}
Compression=lzma
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\gore_mod.exe
WizardStyle=modern
SetupIconFile=..\windows\runner\resources\app_icon.ico
LicenseFile=..\..\..\LICENSE

[UninstallDelete]
; Active settings live under the shared gore-tools umbrella
; (%LOCALAPPDATA%\gore-tools\gore-mod\ui_settings.json). Remove only gore-mod's
; own subfolder so gore-save / gore-cli data under gore-tools survives.
Type: filesandordirs; Name: "{localappdata}\gore-tools\gore-mod"
; Legacy per-app config dir (%APPDATA%\gore-mod), kept only as the one-time
; settings migration source; drop it too.
Type: filesandordirs; Name: "{userappdata}\gore-mod"

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\gore-mod"; Filename: "{app}\gore_mod.exe"
Name: "{autodesktop}\gore-mod"; Filename: "{app}\gore_mod.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional icons:"

[Run]
Filename: "{app}\gore_mod.exe"; Description: "Launch gore-mod"; Flags: nowait postinstall skipifsilent
