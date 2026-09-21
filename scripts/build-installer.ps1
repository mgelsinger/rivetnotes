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
    $default = @(
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 7\ISCC.exe'),
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 7\ISCC.exe')
    ) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    if ($default) {
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
Write-Host "Building rivet $version (release)..."
cargo build --release --bin rivet --locked
if ($LASTEXITCODE -ne 0) { throw 'Rivet build failed.' }

$issPath = Join-Path $repoRoot "installer\rivet.iss"
$noticesDir = Join-Path $repoRoot "THIRD_PARTY_NOTICES"
if (-not (Test-Path $noticesDir)) {
    throw "Missing THIRD_PARTY_NOTICES directory: $noticesDir"
}

& $IsccPath "/DMyAppVersion=$version" "/DMyAppExe=$exePath" $issPath
if ($LASTEXITCODE -ne 0) { throw 'Installer build failed.' }
