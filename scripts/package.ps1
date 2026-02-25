param(
    [string]$OutDir = "dist",
    [ValidateSet("debug", "release")]
    [string]$Configuration = "release"
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

Write-Host "Building rivet $version ($Configuration)..."
cargo build --$Configuration

$binDir = Join-Path $repoRoot "target\$Configuration"
$exePath = Join-Path $binDir "rivet.exe"
if (-not (Test-Path $exePath)) {
    throw "Missing build output: $exePath"
}

$distDir = Join-Path $repoRoot $OutDir
New-Item -ItemType Directory -Force -Path $distDir | Out-Null

$stagingName = "rivet-$version-win64-portable"
$stagingDir = Join-Path $distDir $stagingName
if (Test-Path $stagingDir) {
    Remove-Item -Recurse -Force $stagingDir
}
New-Item -ItemType Directory -Force -Path $stagingDir | Out-Null

Copy-Item $exePath (Join-Path $stagingDir "rivet.exe") -Force
Copy-Item "LICENSE" (Join-Path $stagingDir "LICENSE") -Force
Copy-Item "README.md" (Join-Path $stagingDir "README.md") -Force
if (Test-Path "CHANGELOG.md") {
    Copy-Item "CHANGELOG.md" (Join-Path $stagingDir "CHANGELOG.md") -Force
}
Copy-Item (Join-Path $scriptDir "install.ps1") (Join-Path $stagingDir "install.ps1") -Force
Copy-Item (Join-Path $scriptDir "uninstall.ps1") (Join-Path $stagingDir "uninstall.ps1") -Force

$zipPath = Join-Path $distDir "$stagingName.zip"
if (Test-Path $zipPath) {
    Remove-Item -Force $zipPath
}
Compress-Archive -Path (Join-Path $stagingDir "*") -DestinationPath $zipPath

$checksumPath = Join-Path $distDir "checksums.txt"
$hash = Get-FileHash -Algorithm SHA256 $zipPath
"$($hash.Hash)  $($hash.Path | Split-Path -Leaf)" | Out-File -FilePath $checksumPath -Encoding ascii

Write-Host "Portable zip: $zipPath"
Write-Host "Checksums:   $checksumPath"
