#ifndef MyAppVersion
#define MyAppVersion "0.2.1"
#endif

#ifndef MyArch
#define MyArch "x64"
#endif

[Setup]
AppName=Photon
AppVersion={#MyAppVersion}
DefaultDirName={autopf}\Photon
DefaultGroupName=Photon
OutputDir=output_installer
OutputBaseFilename=Photon-Setup-{#MyArch}-{#MyAppVersion}
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=lowest
; Automatically detect and use the Windows system language
ShowLanguageDialog=auto

[Languages]
; Includes multiple standard language files; Inno Setup picks the matching system language automatically
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
; We don't like French!
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Files]
; Copies the compiled release binaries from the Windows build into the installation directory
Source: "windows_dist\photon_client.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "windows_dist\photon_kernel.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "windows_dist\photon_launcher.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Creates user-friendly shortcuts in the Windows Start Menu
Name: "{group}\Photon Client"; Filename: "{app}\photon_client.exe"
Name: "{group}\Photon Kernel"; Filename: "{app}\photon_kernel.exe"
Name: "{group}\Photon Launcher"; Filename: "{app}\photon_launcher.exe"
Name: "{group}\Uninstall Photon"; Filename: "{uninstallexe}"
