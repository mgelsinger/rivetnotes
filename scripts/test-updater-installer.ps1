param(
    [string]$IsccPath = $env:INNO_SETUP_ISCC,
    [ValidateSet('User', 'Machine')][string]$Scope = 'User'
)
$ErrorActionPreference = 'Stop'
if ($Scope -eq 'Machine') {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'Machine smoke tests require an already elevated test session. This script never requests elevation.'
    }
}
if ([string]::IsNullOrWhiteSpace($IsccPath)) {
    $IsccPath = @(
        (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
        (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 7\ISCC.exe'),
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 7\ISCC.exe'),
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 7\ISCC.exe')
    ) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}
if ([string]::IsNullOrWhiteSpace($IsccPath)) { throw 'Set INNO_SETUP_ISCC to the installed Inno Setup compiler.' }
if (-not (Test-Path -LiteralPath $IsccPath)) { throw 'Set INNO_SETUP_ISCC to the installed Inno Setup compiler.' }
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$id = [Guid]::NewGuid().ToString('N')
$root = Join-Path $repo "target\updater-smoke-$id"
$installation = Join-Path $root 'installed'
if ($Scope -eq 'Machine') {
    $installation = Join-Path $env:ProgramFiles "RivetUpdaterSmoke_$id"
}
$source = Join-Path $root 'rivet.exe'
$mutexName = "RivetUpdaterSmoke_$id"
$hive = if ($Scope -eq 'Machine') { 'HKLM' } else { 'HKCU' }
$otherHive = if ($Scope -eq 'Machine') { 'HKCU' } else { 'HKLM' }
$registryTail = "Software\Microsoft\Windows\CurrentVersion\Uninstall\RivetUpdaterSmoke_$id`_is1"
$registry = "${hive}:\$registryTail"
$otherRegistry = "${otherHive}:\$registryTail"
New-Item -ItemType Directory -Path $root -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $repo 'target\release\rivet.exe') -Destination $source

function Invoke-QuietSetup([string]$File, [string[]]$Arguments) {
    $process = Start-Process -FilePath $File -ArgumentList $Arguments -WindowStyle Hidden -PassThru
    $deadline = [DateTime]::UtcNow.AddMinutes(2)
    while (-not $process.WaitForExit(50)) {
        $process.Refresh()
        if ($process.MainWindowHandle -ne 0) { throw 'Silent setup displayed a window.' }
        if ([DateTime]::UtcNow -gt $deadline) { throw 'Silent setup timed out.' }
    }
    return $process.ExitCode
}

try {
    $scopeSwitch = if ($Scope -eq 'Machine') { '/ALLUSERS' } else { '/CURRENTUSER' }
    $common = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/SP-', '/NORESTART', '/NOCLOSEAPPLICATIONS', '/NORESTARTAPPLICATIONS', '/RESTARTEXITCODE=3010', $scopeSwitch, "/DIR=`"$installation`"")
    foreach ($version in @('0.0.1', '0.0.2')) {
        # Different bytes prove that the second installer replaces the payload.
        $stream = [IO.File]::OpenWrite($source)
        try { $null = $stream.Seek(0, [IO.SeekOrigin]::End); $stream.WriteByte(1) } finally { $stream.Dispose() }
        & $IsccPath "/DMyAppVersion=$version" "/DMyAppExe=$source" "/DMyAppName=RivetUpdaterSmoke_$id" "/DMyAppId=RivetUpdaterSmoke_$id" "/DMyAppMutex=$mutexName" '/DUpdaterSmokeTest' "/O$root" (Join-Path $repo 'installer\rivet.iss')
        if ($LASTEXITCODE -ne 0) { throw 'Smoke installer compilation failed.' }
        $setup = Join-Path $root "rivet-$version-setup.exe"
        # Match the production helper's read-only lock during verification/run.
        $locked = [IO.File]::Open($setup, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
        try {
            if ((Invoke-QuietSetup $setup ($common + "/LOG=`"$root\install-$version.log`"")) -ne 0) { throw 'Silent installation failed.' }
        } finally { $locked.Dispose() }
        if ((Get-ItemProperty -LiteralPath $registry).DisplayVersion -ne $version) { throw 'Installed version is incorrect.' }
        if (Test-Path -LiteralPath $otherRegistry) { throw 'Update changed the installation scope.' }
        if ((Get-FileHash -LiteralPath $source).Hash -ne (Get-FileHash -LiteralPath (Join-Path $installation 'rivet.exe')).Hash) { throw 'Installed payload was not replaced.' }
        if (Get-Process rivet -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq (Join-Path $installation 'rivet.exe') }) { throw 'Silent installation unexpectedly launched Rivet.' }
    }
    $mutex = [Threading.Mutex]::new($false, $mutexName)
    try {
        if ((Invoke-QuietSetup $setup ($common + "/LOG=`"$root\mutex.log`"")) -eq 0) { throw 'Installer ignored the running-app mutex.' }
    } finally { $mutex.Dispose() }
    $setupMutex = [Threading.Mutex]::new($false, "Global\$($mutexName)_Setup")
    try {
        if ((Invoke-QuietSetup $setup ($common + "/LOG=`"$root\setup-mutex.log`"")) -eq 0) { throw 'Installer ignored the shared setup mutex.' }
    } finally { $setupMutex.Dispose() }
    Write-Host "$Scope scope: silent install/upgrade, file lock, no relaunch, and app/setup mutex checks passed."
} finally {
    $uninstaller = Join-Path $installation 'unins000.exe'
    if (Test-Path -LiteralPath $uninstaller) {
        $code = Invoke-QuietSetup $uninstaller @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART')
        if ($code -ne 0) { Write-Warning "Isolated smoke-test cleanup failed: $code" }
    }
    # Keep only this test's files/logs for diagnosis; never touch the real Rivet
    # registry entries, shortcuts, installed program, or user-data directory.
}
