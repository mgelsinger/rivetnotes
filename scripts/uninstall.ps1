param()

$ErrorActionPreference = "Stop"

$installDir = Join-Path $env:LOCALAPPDATA "Rivet"
$targetExe = Join-Path $installDir "Rivet.exe"
# The legacy script installs alongside notes, backups, and preferences. Never
# recursively remove this shared directory when uninstalling the executable.
if (Test-Path -LiteralPath $targetExe) {
    Remove-Item -LiteralPath $targetExe -Force
}

$shortcutPath = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\Rivet.lnk"
if (Test-Path -LiteralPath $shortcutPath) {
    $shortcut = (New-Object -ComObject WScript.Shell).CreateShortcut($shortcutPath)
    if ($shortcut.TargetPath -eq $targetExe) {
        Remove-Item -LiteralPath $shortcutPath -Force
    }
}

Write-Host "Uninstalled the script-installed Rivet executable. Notes, backups, settings, and other installations were preserved."
