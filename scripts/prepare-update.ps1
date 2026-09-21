param([Parameter(Mandatory)][string]$Tag)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Push-Location $repo
try {
    $match = Select-String -Path Cargo.toml -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
    $version = $match.Matches[0].Groups[1].Value
    if ($Tag -cne "v$version" -or $version -notmatch '^\d+\.\d+\.\d+$') { throw 'Release tag must match the stable Cargo version.' }
    if ([string]::IsNullOrWhiteSpace($env:RIVET_UPDATE_SIGNING_KEY)) { throw 'Configure the RIVET_UPDATE_SIGNING_KEY repository secret before releasing.' }
    cargo run --locked --bin update-release -- sign $version "dist/rivet-$version-setup.exe" dist/update.json
    if ($LASTEXITCODE -ne 0) { throw 'Update signing failed.' }
    cargo run --locked --bin update-release -- verify dist/update.json "dist/rivet-$version-setup.exe"
    if ($LASTEXITCODE -ne 0) { throw 'Signed update verification failed.' }
    $files = @("dist/rivet-$version-setup.exe", "dist/rivet-$version-win64-portable.zip", 'dist/update.json')
    $checksums = foreach ($file in $files) {
        $hash = Get-FileHash -LiteralPath $file -Algorithm SHA256
        "$($hash.Hash)  $([IO.Path]::GetFileName($hash.Path))"
    }
    $checksums | Set-Content -LiteralPath dist/checksums.txt -Encoding ascii
} finally { Pop-Location }
