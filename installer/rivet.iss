#ifndef MyAppName
  #define MyAppName "Rivet"
#endif
#ifndef MyAppId
  #define MyAppId "{{66885411-0CEF-459E-AA39-4B257B1A4D84}}"
#endif
#ifndef MyAppMutex
  #define MyAppMutex "RivetNotes_SingleInstance_66885411-0CEF-459E-AA39-4B257B1A4D84"
#endif

#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif

#ifndef MyAppExe
  #define MyAppExe "..\\target\\release\\rivet.exe"
#endif

[Setup]
AppId={#MyAppId}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\dist
OutputBaseFilename=rivet-{#MyAppVersion}-setup
SetupIconFile=..\assets\rivet.ico
UninstallDisplayIcon={app}\rivet.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ChangesAssociations=yes
; Never replace an editor that was reopened while an update was being prepared.
AppMutex={#MyAppMutex}
; Serialize installation across Windows sessions, including shared installs.
SetupMutex=Global\{#MyAppMutex}_Setup
CloseApplications=no
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop icon"; GroupDescription: "Additional icons"; Flags: unchecked

[Files]
Source: "{#MyAppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\\NOTICE.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\\CHANGELOG.md"; DestDir: "{app}"; Flags: ignoreversion
; Copy the full notices directory so each bundled third-party component ships with its own notice file.
Source: "..\\THIRD_PARTY_NOTICES\\*"; DestDir: "{app}\\THIRD_PARTY_NOTICES"; Flags: ignoreversion recursesubdirs createallsubdirs

[Registry]
#ifndef UpdaterSmokeTest
; ProgID used by "Open with" and Settings > Default apps file associations.
Root: HKA; Subkey: "Software\Classes\Rivet.Document"; ValueType: string; ValueName: ""; ValueData: "Rivet Document"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\Rivet.Document\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\rivet.exe"",0"
Root: HKA; Subkey: "Software\Classes\Rivet.Document\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\rivet.exe"" ""%1"""

; "Open with Rivet" in the Explorer context menu for every file type.
Root: HKA; Subkey: "Software\Classes\*\shell\OpenWithRivet"; ValueType: string; ValueName: ""; ValueData: "Open with Rivet"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\*\shell\OpenWithRivet"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\rivet.exe"",0"
Root: HKA; Subkey: "Software\Classes\*\shell\OpenWithRivet\command"; ValueType: string; ValueName: ""; ValueData: """{app}\rivet.exe"" ""%1"""

; Application registration for the "Open with" dialog.
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "Rivet"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: """{app}\rivet.exe"",0"
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\rivet.exe"" ""%1"""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".txt"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".md"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".markdown"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".log"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".json"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".xml"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".yaml"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".yml"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".ini"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".cfg"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".conf"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".csv"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".toml"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".ps1"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".psm1"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".py"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".rs"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".html"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".css"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".js"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".ts"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".c"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".h"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".cpp"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".sh"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".bat"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\rivet.exe\SupportedTypes"; ValueType: string; ValueName: ".cmd"; ValueData: ""

; Default Programs capabilities so Rivet appears in Settings > Default apps.
Root: HKA; Subkey: "Software\Rivet"; Flags: uninsdeletekeyifempty
Root: HKA; Subkey: "Software\Rivet\Capabilities"; ValueType: string; ValueName: "ApplicationName"; ValueData: "Rivet"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Rivet\Capabilities"; ValueType: string; ValueName: "ApplicationDescription"; ValueData: "Fast Windows-native text editor"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".txt"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".md"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".markdown"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".log"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".json"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".xml"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".yaml"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".yml"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".ini"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".cfg"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".conf"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".csv"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".toml"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".ps1"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".psm1"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".py"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".rs"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".html"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".css"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".js"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".ts"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".c"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".h"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".cpp"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".sh"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".bat"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\Rivet\Capabilities\FileAssociations"; ValueType: string; ValueName: ".cmd"; ValueData: "Rivet.Document"
Root: HKA; Subkey: "Software\RegisteredApplications"; ValueType: string; ValueName: "Rivet"; ValueData: "Software\Rivet\Capabilities"; Flags: uninsdeletevalue
#endif

[Icons]
#ifndef UpdaterSmokeTest
Name: "{autoprograms}\Rivet"; Filename: "{app}\rivet.exe"
Name: "{autodesktop}\Rivet"; Filename: "{app}\rivet.exe"; Tasks: desktopicon
#endif

[Run]
Filename: "{app}\rivet.exe"; Description: "Launch Rivet"; Flags: nowait postinstall skipifsilent
