#define AppName "Widno"
#define AppVersion "1.0.0"
#define AppPublisher "Xander-Lein"
#define AppURL "https://github.com/Xander-Lein/widno"
#define AppExeName "widno.exe"

[Setup]
AppId={{A8D1A4A2-8B2C-4F8D-9F58-7D6C0B8C9E10}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}/issues
DefaultDirName={localappdata}\Programs\Widno
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\..\dist
OutputBaseFilename=Widno-{#AppVersion}-Windows-x64-Setup
SetupIconFile=..\assets\widno-icon.ico
UninstallDisplayIcon={app}\{#AppExeName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
VersionInfoVersion={#AppVersion}.0
VersionInfoDescription=Widno Windows image viewer
VersionInfoCopyright=Copyright (c) 2026 Xander-Lein
LicenseFile=..\LICENSE

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"
Name: "startmenuicon"; Description: "Create a Start Menu shortcut"; GroupDescription: "Additional shortcuts:"; Flags: checkedonce

[Files]
Source: "..\target\release\widno.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\scripts\register-file-associations.ps1"; DestDir: "{app}\scripts"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Widno"; Filename: "{app}\{#AppExeName}"; Tasks: startmenuicon
Name: "{autodesktop}\Widno"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExeName}"; Description: "Launch Widno"; Flags: nowait postinstall skipifsilent
