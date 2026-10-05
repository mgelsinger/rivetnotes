param(
    [switch]$Offline,
    [switch]$Check
)

$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
Push-Location $repo
try {
    $metadataArguments = @('metadata', '--format-version', '1', '--locked', '--filter-platform', 'x86_64-pc-windows-msvc')
    if ($Offline) { $metadataArguments += '--offline' }
    $metadataJson = & cargo @metadataArguments
    if ($LASTEXITCODE -ne 0) { throw 'Could not read locked Windows x64 dependency metadata.' }
    $metadata = ($metadataJson -join "`n") | ConvertFrom-Json
    $packages = @($metadata.packages | Where-Object { $_.source } | Sort-Object name, version)
    if ($packages.Count -eq 0) { throw 'No dependency packages found.' }
    $lockText = [IO.File]::ReadAllText((Join-Path $repo 'Cargo.lock')).Replace("`r`n", "`n").Replace("`r", "`n")
    $lockHash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($lockText))).ToLowerInvariant()
    $report = [Text.StringBuilder]::new()
    [void]$report.AppendLine('Rivet Rust dependency attributions')
    [void]$report.AppendLine('')
    [void]$report.AppendLine('Target: x86_64-pc-windows-msvc')
    [void]$report.AppendLine("Cargo.lock SHA-256 (LF line endings): $lockHash")
    [void]$report.AppendLine("Third-party packages: $($packages.Count)")
    [void]$report.AppendLine('')
    [void]$report.AppendLine('This report includes every third-party package returned by locked Cargo')
    [void]$report.AppendLine('metadata for Windows x64, including build and development dependencies.')
    [void]$report.AppendLine('Inclusion does not imply that every package is linked into the application.')
    [void]$report.AppendLine('License and notice texts below are copied from the corresponding crate')
    [void]$report.AppendLine('sources, with LF line endings and trailing spaces removed. No alternatives have')
    [void]$report.AppendLine('been removed. Regenerate with scripts/generate-third-party-notices.ps1.')
    [void]$report.AppendLine('')
    [void]$report.AppendLine('Package index:')
    foreach ($package in $packages) {
        [void]$report.AppendLine("- $($package.name) $($package.version): $($package.license)")
    }
    foreach ($package in $packages) {
        $sourceDirectory = Split-Path -Parent $package.manifest_path
        $licenseFiles = @(Get-ChildItem -LiteralPath $sourceDirectory -File | Where-Object {
            $_.Name -match '^(LICENSE|LICENCE|COPYING|UNLICENSE|COPYRIGHT|NOTICE)([.\-_]|$)'
        } | Select-Object -ExpandProperty FullName)
        if ($package.license_file) {
            $licenseFiles += [IO.Path]::GetFullPath((Join-Path $sourceDirectory $package.license_file))
        }
        $licenseFiles = @($licenseFiles | Sort-Object -Unique)
        if ($licenseFiles.Count -eq 0) { throw "No license texts found for $($package.name) $($package.version)." }
        [void]$report.AppendLine('')
        [void]$report.AppendLine(('=' * 78))
        [void]$report.AppendLine("$($package.name) $($package.version)")
        [void]$report.AppendLine("License expression: $($package.license)")
        [void]$report.AppendLine("Source: https://crates.io/crates/$($package.name)/$($package.version)")
        if ($package.repository) { [void]$report.AppendLine("Repository: $($package.repository)") }
        if ($package.authors.Count -gt 0) { [void]$report.AppendLine("Authors: $($package.authors -join '; ')") }
        foreach ($licenseFile in $licenseFiles) {
            $licenseText = [IO.File]::ReadAllText($licenseFile).Replace("`r`n", "`n").Replace("`r", "`n").TrimEnd([char[]]"`r`n")
            if ([string]::IsNullOrWhiteSpace($licenseText)) { throw "Empty license text: $licenseFile" }
            [void]$report.AppendLine('')
            [void]$report.AppendLine("File: $([IO.Path]::GetRelativePath($sourceDirectory, $licenseFile).Replace('\', '/'))")
            [void]$report.AppendLine(('-' * 78))
            [void]$report.AppendLine($licenseText)
        }
    }
    $text = $report.ToString().Replace("`r`n", "`n") -replace '(?m)[ \t]+$', ''
    $output = Join-Path $repo 'THIRD_PARTY_NOTICES/Rust-Crates.txt'
    if ($Check) {
        if (-not (Test-Path -LiteralPath $output) -or [IO.File]::ReadAllText($output).Replace("`r`n", "`n").Replace("`r", "`n") -cne $text) {
            throw 'Rust dependency attributions are missing or out of date. Run this script without -Check.'
        }
        Write-Host "Verified attributions for $($packages.Count) Windows x64 dependency packages."
    } else {
        [IO.File]::WriteAllText($output, $text, [Text.UTF8Encoding]::new($false))
        Write-Host "Wrote attributions for $($packages.Count) Windows x64 dependency packages."
    }
} finally {
    Pop-Location
}
