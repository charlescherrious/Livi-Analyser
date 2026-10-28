; Livi Analyser by CC Audio: one setup.exe with tick boxes.
; Built by the GitHub workflow (see .github/workflows/build.yml).
; Expects, next to this file's "build" folder:
;   build\app-setup.exe            (the desktop app installer made by Tauri)
;   build\Livi Analyser.vst3\...   (the plugin folder made by cargo xtask)

#define AppName "Livi Analyser"
#define AppVersion "0.2.0"

[Setup]
AppId={{6B1D2C58-3B7A-4C6B-9E0B-5C1A4C0A11A1}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=CC Audio
DefaultDirName={commonpf64}\CC Audio\{#AppName}
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=output
OutputBaseFilename=LiviAnalyser-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayName={#AppName} (CC Audio plugin)

[Types]
Name: "full"; Description: "Install everything"
Name: "custom"; Description: "Choose what to install"; Flags: iscustom

[Components]
Name: "app"; Description: "Desktop app (drag a song in, get BPM and key)"; Types: full custom
Name: "vst3"; Description: "VST3 plugin for FL Studio, Ableton, Reaper and others (listed under CC Audio)"; Types: full custom

[Files]
; The desktop app is installed by its own installer, run silently below.
Source: "build\app-setup.exe"; DestDir: "{tmp}"; Components: app; Flags: deleteafterinstall
; The plugin goes in the standard VST3 folder, inside a "CC Audio" folder.
Source: "build\Livi Analyser.vst3\*"; DestDir: "{commoncf64}\VST3\CC Audio\Livi Analyser.vst3"; Components: vst3; Flags: recursesubdirs createallsubdirs ignoreversion

[Run]
Filename: "{tmp}\app-setup.exe"; Parameters: "/S"; Components: app; StatusMsg: "Installing the desktop app..."; Flags: waituntilterminated

[UninstallDelete]
Type: filesandordirs; Name: "{commoncf64}\VST3\CC Audio\Livi Analyser.vst3"
