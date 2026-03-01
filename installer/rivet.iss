#define MyAppName "Rivet"
#define MyAppId "{{66885411-0CEF-459E-AA39-4B257B1A4D84}}"

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
DefaultDirName={autopf}\Rivet
DefaultGroupName=Rivet
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
Source: "..\\THIRD_PARTY_NOTICES\\*"; DestDir: "{app}\\THIRD_PARTY_NOTICES"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Rivet"; Filename: "{app}\rivet.exe"
Name: "{autodesktop}\Rivet"; Filename: "{app}\rivet.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\rivet.exe"; Description: "Launch Rivet"; Flags: nowait postinstall skipifsilent
