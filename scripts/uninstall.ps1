param()

$ErrorActionPreference = "Stop"

$installDir = Join-Path $env:LOCALAPPDATA "Rivet"
$targetExe = Join-Path $installDir "Rivet.exe"
if (Test-Path $targetExe) {
    Remove-Item -Force $targetExe
}
if (Test-Path $installDir) {
    Remove-Item -Recurse -Force $installDir
}

$shortcutPath = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Rivet.lnk"
if (Test-Path $shortcutPath) {
    Remove-Item -Force $shortcutPath
}

Write-Host "Uninstalled Rivet"
