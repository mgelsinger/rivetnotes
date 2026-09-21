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
if ($Configuration -eq 'release') { cargo build --release --bin rivet --locked }
else { cargo build --bin rivet --locked }
if ($LASTEXITCODE -ne 0) { throw 'Rivet build failed.' }

$binDir = Join-Path $repoRoot "target\$Configuration"
$exePath = Join-Path $binDir "rivet.exe"
if (-not (Test-Path $exePath)) {
    throw "Missing build output: $exePath"
}

$distDir = Join-Path $repoRoot $OutDir
New-Item -ItemType Directory -Force -Path $distDir | Out-Null

$noticesDir = Join-Path $repoRoot "THIRD_PARTY_NOTICES"
if (-not (Test-Path $noticesDir)) {
    throw "Missing THIRD_PARTY_NOTICES directory: $noticesDir"
}
if (-not (Get-ChildItem -Path $noticesDir -Recurse -File | Select-Object -First 1)) {
    throw "THIRD_PARTY_NOTICES is empty: $noticesDir"
}

$stagingName = "rivet-$version-win64-portable"
$stagingDir = Join-Path $distDir $stagingName
if (Test-Path $stagingDir) {
    $resolvedStaging = (Resolve-Path -LiteralPath $stagingDir).Path
    $resolvedDist = (Resolve-Path -LiteralPath $distDir).Path
    if ((Split-Path -Parent $resolvedStaging) -ne $resolvedDist) { throw 'Invalid staging directory.' }
    Remove-Item -LiteralPath $resolvedStaging -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $stagingDir | Out-Null

Copy-Item $exePath (Join-Path $stagingDir "rivet.exe") -Force
Copy-Item "LICENSE" (Join-Path $stagingDir "LICENSE") -Force
Copy-Item "NOTICE.txt" (Join-Path $stagingDir "NOTICE.txt") -Force
Copy-Item "README.md" (Join-Path $stagingDir "README.md") -Force
if (Test-Path "CHANGELOG.md") {
    Copy-Item "CHANGELOG.md" (Join-Path $stagingDir "CHANGELOG.md") -Force
}
Copy-Item (Join-Path $scriptDir "install.ps1") (Join-Path $stagingDir "install.ps1") -Force
Copy-Item (Join-Path $scriptDir "uninstall.ps1") (Join-Path $stagingDir "uninstall.ps1") -Force
Copy-Item $noticesDir (Join-Path $stagingDir "THIRD_PARTY_NOTICES") -Recurse -Force

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
