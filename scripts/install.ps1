param(
    [string]$SourceDir = (Split-Path -Parent $MyInvocation.MyCommand.Path)
)

$ErrorActionPreference = "Stop"

$exePath = Join-Path $SourceDir "rivet.exe"
if (-not (Test-Path $exePath)) {
    throw "rivet.exe not found in $SourceDir"
}

$installDir = Join-Path $env:LOCALAPPDATA "Rivet"
New-Item -ItemType Directory -Force -Path $installDir | Out-Null

$targetExe = Join-Path $installDir "Rivet.exe"
Copy-Item $exePath $targetExe -Force

$shortcutDir = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs"
$shortcutPath = Join-Path $shortcutDir "Rivet.lnk"
$wsh = New-Object -ComObject WScript.Shell
$shortcut = $wsh.CreateShortcut($shortcutPath)
$shortcut.TargetPath = $targetExe
$shortcut.WorkingDirectory = $installDir
$shortcut.Save()

Write-Host "Installed to $installDir"
