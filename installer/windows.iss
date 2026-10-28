; One installer with tick boxes: desktop app and/or VST3 plugin (CC Audio).
; Built in CI by .github/workflows/build.yml. Expects installer/payload/ to hold:
;   desktop-setup.exe       the Tauri (NSIS) installer for the desktop app
;   vst3\Livi Analyser.vst3 the plugin bundle (a folder)

#define AppName "Livi Analyser"
#define AppVersion "0.2.0"

[Setup]
AppId={{6F1D2C4E-8B7A-4E55-9C3A-4C43A0D1A5E1}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=CC Audio
DefaultDirName={autopf}\CC Audio\{#AppName}
DefaultGroupName=CC Audio
DisableProgramGroupPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=Output
OutputBaseFilename=Livi-Analyser-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
DisableDirPage=yes

[Types]
Name: "full"; Description: "Desktop app and VST3 plugin"
Name: "custom"; Description: "Choose what to install"; Flags: iscustom

[Components]
Name: "desktop"; Description: "Desktop app"; Types: full custom
Name: "vst3"; Description: "VST3 plugin (listed under CC Audio in your DAW)"; Types: full custom

[Files]
Source: "payload\desktop-setup.exe"; DestDir: "{tmp}"; Flags: deleteafterinstall; Components: desktop
Source: "payload\vst3\Livi Analyser.vst3\*"; DestDir: "{commoncf64}\VST3\CC Audio\Livi Analyser.vst3"; Flags: recursesubdirs createallsubdirs ignoreversion; Components: vst3

[Run]
; The desktop app installs through its own installer (it sets up WebView2 if needed).
Filename: "{tmp}\desktop-setup.exe"; Parameters: "/S"; StatusMsg: "Installing the desktop app..."; Flags: waituntilterminated; Components: desktop

[Code]
procedure CurPageChanged(CurPageID: Integer);
begin
  if CurPageID = wpFinished then
    WizardForm.FinishedLabel.Caption :=
      'Livi Analyser is installed.' + #13#10#13#10 +
      'If you installed the plugin: in FL Studio open Options > Manage plugins and click "Find installed plugins". It appears under CC Audio.';
end;
