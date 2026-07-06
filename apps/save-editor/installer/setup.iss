; goresave Windows installer.
; Compiled by CI:
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
AppId={{C7A35D8E-4B61-4E0D-9C0A-2F8B5D1E6A43}
AppName=GORE Save Editor
AppVersion={#AppVersion}
AppVerName=GORE Save Editor {#AppVersion}
AppPublisher=dh0er
DefaultDirName={autopf}\goresave
DefaultGroupName=GORE Save Editor
; Per-user installs work without elevation; the dialog lets the user pick
; all-users (Program Files, admin) or current-user (no UAC prompt).
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir={#OutputDir}
OutputBaseFilename=GoresaveSetup-{#AppVersion}
Compression=lzma
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\goresave.exe
WizardStyle=modern
SetupIconFile=..\windows\runner\resources\app_icon.ico
LicenseFile=..\..\..\LICENSE

; Remove obsolete files from earlier versions before copying the new bundle.
; Inno only adds/overwrites bundle files; it never prunes files that were
; dropped from a release, so stale binaries must be deleted explicitly.
[InstallDelete]
; The out-of-process G1R codec host was replaced by an in-process codec
; (linked into gore_save.dll); remove the now-unused helper on upgrade.
Type: files; Name: "{app}\goresave_g1r_codec_host.exe"
; ...and its derived-profile cache. The host defaulted to %LOCALAPPDATA%; also
; clear the %APPDATA% location defensively in case an older build wrote there.
Type: files; Name: "{localappdata}\goresave\g1r_codec_host_derived_profiles.json"
Type: files; Name: "{userappdata}\goresave\g1r_codec_host_derived_profiles.json"

[UninstallDelete]
; Remove goresave's per-user config/data on uninstall (settings.json,
; ui_settings.json). Save backups live next to the saves, not here.
Type: filesandordirs; Name: "{userappdata}\goresave"
; Stale codec-host cache lived under %LOCALAPPDATA%; drop just that file (the
; rest of %LOCALAPPDATA%\goresave may belong to a different installer).
Type: files; Name: "{localappdata}\goresave\g1r_codec_host_derived_profiles.json"

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
; LICENSE + third-party attributions live at the repo root, not in SourceDir
; (the Flutter Release dir), so include them explicitly. Paths are relative to
; this .iss (apps\<app>\installer\ -> repo root is ..\..\..).
Source: "..\..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\..\THIRD_PARTY_LICENSES.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\GORE Save Editor"; Filename: "{app}\goresave.exe"
Name: "{autodesktop}\GORE Save Editor"; Filename: "{app}\goresave.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional icons:"

[Run]
Filename: "{app}\goresave.exe"; Description: "Launch GORE Save Editor"; Flags: nowait postinstall skipifsilent
