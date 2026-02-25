param(
    [string]$IsccPath = $env:INNO_SETUP_ISCC
)

$ErrorActionPreference = "Stop"

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$repoRoot = Resolve-Path (Join-Path $scriptDir "..")
Set-Location $repoRoot

$versionMatch = Select-String -Path "Cargo.toml" -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
if (-not $versionMatch) {
    throw "Unable to read version from Cargo.toml."
}
$version = $versionMatch.Matches[0].Groups[1].Value

if ([string]::IsNullOrWhiteSpace($IsccPath)) {
    $default = Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe"
    if (Test-Path $default) {
        $IsccPath = $default
    } else {
        $cmd = Get-Command iscc.exe -ErrorAction SilentlyContinue
        if ($cmd) {
            $IsccPath = $cmd.Source
        } else {
            throw "ISCC.exe not found. Install Inno Setup or set INNO_SETUP_ISCC."
        }
    }
}

$exePath = Join-Path $repoRoot "target\release\rivet.exe"
if (-not (Test-Path $exePath)) {
    Write-Host "Building rivet $version (release)..."
    cargo build --release
}

$issPath = Join-Path $repoRoot "installer\rivet.iss"
& $IsccPath "/DMyAppVersion=$version" "/DMyAppExe=$exePath" $issPath
