; TaskbarMetrics-Setup-<version>.exe. Built by tools\release.ps1, which passes
; AppVersion, Source (the package folder), Icon and OutputDir.
#ifndef AppVersion
  #error Build with tools\release.ps1
#endif

#define PawnIOUrl "https://github.com/namazso/PawnIO.Setup/releases/download/2.2.0/PawnIO_setup.exe"
#define PawnIOSha256 "1f519a22e47187f70a1379a48ca604981c4fcf694f4e65b734aaa74a9fba3032"

[Setup]
AppId={{D792678F-12CE-4409-8799-DA4FF9AE5029}
AppName=Taskbar Metrics
AppVersion={#AppVersion}
VersionInfoVersion={#AppVersion}
; Program Files only: the collector tasks elevate files that users cannot change.
DefaultDirName={autopf}\Taskbar Metrics
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
MinVersion=10.0.22000
OutputDir={#OutputDir}
OutputBaseFilename=TaskbarMetrics-Setup-{#AppVersion}
SetupIconFile={#Icon}
UninstallDisplayIcon={app}\TaskbarMetrics.exe
UninstallDisplayName=Taskbar Metrics
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
; Explorer holds the DLL; --unload frees the files instead of the Restart Manager.
CloseApplications=no
; The Run value belongs to the user who installs, as the window's own switch does.
UsedUserAreasWarning=no

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl,en.isl"
Name: "ru"; MessagesFile: "compiler:Languages\Russian.isl,ru.isl"

[Tasks]
Name: "autostart"; Description: "{cm:TaskAutostart}"
Name: "pawnio"; Description: "{cm:TaskPawnIO}"; Check: not PawnIOInstalled

[Files]
Source: "{#Source}\TaskbarMetrics.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Source}\TaskbarMetrics.Window.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Source}\TaskbarMetrics.History.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Source}\TaskbarMetrics.Sensors.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Source}\TaskbarMetrics.Host.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Source}\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Source}\pawnio\*.bin"; DestDir: "{app}\pawnio"; Flags: ignoreversion
Source: "{#Source}\pawnio\COPYING"; DestDir: "{app}\pawnio"; Flags: ignoreversion
Source: "{#Source}\pawnio\README.md"; DestDir: "{app}\pawnio"; Flags: ignoreversion
; LGPL: the modules travel with their corresponding source.
Source: "{#Source}\pawnio\source\PawnIO.Modules-0.2.11.zip"; DestDir: "{app}\pawnio\source"; Flags: ignoreversion

[Icons]
; Start menu search finds the program by this shortcut.
Name: "{autoprograms}\Taskbar Metrics"; Filename: "{app}\TaskbarMetrics.exe"

[Registry]
; The same command the window's "Start with Windows" switch writes.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Taskbar Metrics"; ValueData: """{app}\TaskbarMetrics.exe"" --autostart"; Tasks: autostart

[Run]
Filename: "{app}\TaskbarMetrics.exe"; Parameters: "--register-tasks"; Flags: runhidden waituntilterminated
; --autostart retries while a restarted taskbar is not ready yet.
Filename: "{app}\TaskbarMetrics.exe"; Parameters: "--autostart"; Description: "{cm:Launch}"; Flags: nowait postinstall runasoriginaluser runhidden

[UninstallRun]
Filename: "{app}\TaskbarMetrics.exe"; Parameters: "--unload"; Flags: runhidden waituntilterminated; RunOnceId: "Unload"
Filename: "{app}\TaskbarMetrics.exe"; Parameters: "--remove-tasks"; Flags: runhidden waituntilterminated; RunOnceId: "RemoveTasks"

[Code]
var
  DownloadPage: TDownloadWizardPage;

function PawnIOInstalled: Boolean;
begin
  Result := RegKeyExists(HKLM64, 'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO');
end;

procedure InitializeWizard;
begin
  DownloadPage := CreateDownloadPage(SetupMessage(msgWizardPreparing), SetupMessage(msgPreparingDesc), nil);
end;

{ The driver's official installer, checked against its SHA-256. }
function NextButtonClick(CurPageID: Integer): Boolean;
begin
  Result := True;
  if (CurPageID = wpReady) and WizardIsTaskSelected('pawnio') then begin
    DownloadPage.Clear;
    DownloadPage.Add('{#PawnIOUrl}', 'PawnIO_setup.exe', '{#PawnIOSha256}');
    DownloadPage.Show;
    try
      try
        DownloadPage.Download;
      except
        SuppressibleMsgBox(FmtMessage(CustomMessage('PawnIOFailed'), [GetExceptionMessage]), mbError, MB_OK, IDOK);
      end;
    finally
      DownloadPage.Hide;
    end;
  end;
end;

{ An update first frees the files that Explorer and the window hold. }
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Code: Integer;
  Launcher: String;
begin
  Result := '';
  Launcher := ExpandConstant('{app}\TaskbarMetrics.exe');
  if FileExists(Launcher) then
    Exec(Launcher, '--unload', '', SW_HIDE, ewWaitUntilTerminated, Code);
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  Code: Integer;
  Installer: String;
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('pawnio') then begin
    Installer := ExpandConstant('{tmp}\PawnIO_setup.exe');
    if not FileExists(Installer) then
      Exit;
    WizardForm.StatusLabel.Caption := CustomMessage('InstallingPawnIO');
    if not Exec(Installer, '-install -silent', '', SW_HIDE, ewWaitUntilTerminated, Code) or (Code <> 0) then
      SuppressibleMsgBox(FmtMessage(CustomMessage('PawnIOFailed'), [IntToStr(Code)]), mbError, MB_OK, IDOK);
  end;
end;

{ The Run value may also come from the window's switch; it goes when it starts this copy. }
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Command: String;
begin
  if (CurUninstallStep = usPostUninstall) and
     RegQueryStringValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'Taskbar Metrics', Command) and
     (Pos(Lowercase(ExpandConstant('{app}\')), Lowercase(Command)) > 0) then
    RegDeleteValue(HKCU, 'Software\Microsoft\Windows\CurrentVersion\Run', 'Taskbar Metrics');
end;
