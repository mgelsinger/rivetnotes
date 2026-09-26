param()
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$root = Join-Path $repo ('target\legacy-uninstall-' + [Guid]::NewGuid().ToString('N'))
$savedLocal = $env:LOCALAPPDATA
$savedRoaming = $env:APPDATA
try {
    $env:LOCALAPPDATA = Join-Path $root 'local'
    $env:APPDATA = Join-Path $root 'roaming'
    $data = Join-Path $env:LOCALAPPDATA 'Rivet'
    $backup = Join-Path $data 'backup\note.bak'
    $shortcutPath = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Rivet.lnk'
    New-Item -ItemType Directory -Path (Split-Path $backup), (Split-Path $shortcutPath) -Force | Out-Null
    $exe = Join-Path $data 'Rivet.exe'
    [IO.File]::WriteAllText($backup, 'precious unsaved note')
    [IO.File]::WriteAllText((Join-Path $data 'settings.json'), '{"dark_mode":true}')
    [IO.File]::WriteAllText($exe, 'legacy executable fixture')
    $shell = New-Object -ComObject WScript.Shell
    $shortcut = $shell.CreateShortcut($shortcutPath)
    $shortcut.TargetPath = Join-Path $root 'another-install\rivet.exe'
    $shortcut.Save()
    & (Join-Path $PSScriptRoot 'uninstall.ps1')
    if (Test-Path -LiteralPath $exe) { throw 'Legacy executable was not removed.' }
    if (-not (Test-Path -LiteralPath $shortcutPath)) { throw 'Another installation lost its shortcut.' }
    if ([IO.File]::ReadAllText($backup) -cne 'precious unsaved note') { throw 'Backup contents changed.' }
    if ([IO.File]::ReadAllText((Join-Path $data 'settings.json')) -cne '{"dark_mode":true}') { throw 'Settings changed.' }
    $shortcut.TargetPath = $exe
    $shortcut.Save()
    & (Join-Path $PSScriptRoot 'uninstall.ps1')
    if (Test-Path -LiteralPath $shortcutPath) { throw 'Legacy shortcut was not removed.' }
    if (-not (Test-Path -LiteralPath $backup)) { throw 'Repeated uninstall removed notes.' }
    Write-Host 'Legacy uninstall: notes, settings, and unrelated shortcuts preserved.'
} finally {
    $env:LOCALAPPDATA = $savedLocal
    $env:APPDATA = $savedRoaming
    # Keep isolated fixtures under target for inspection. Never touch real data.
}
