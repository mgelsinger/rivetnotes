param([Parameter(Mandatory)][string]$Tag)
$ErrorActionPreference = 'Stop'
if ($Tag -notmatch '^v\d+\.\d+\.\d+$') { throw 'A stable version tag is required.' }
$version = $Tag.Substring(1)
$existing = gh release view $Tag --json isDraft 2>$null
if ($LASTEXITCODE -eq 0) {
    if (-not ($existing | ConvertFrom-Json).isDraft) { throw 'Refusing to replace a published release. Publish a new version instead.' }
} else {
    gh release create $Tag --verify-tag --draft --title "Rivet $version" --notes-file dist/release-notes.md
    if ($LASTEXITCODE -ne 0) { throw 'Could not create draft release.' }
}
$files = @("dist/rivet-$version-setup.exe", "dist/rivet-$version-win64-portable.zip", 'dist/update.json', 'dist/checksums.txt')
gh release upload $Tag @files --clobber
if ($LASTEXITCODE -ne 0) { throw 'Release upload failed; release remains a draft.' }
# Publish only after every required asset is present. Clients use the latest
# published release, so an incomplete upload cannot become their update feed.
$release = gh release view $Tag --json assets
if ($LASTEXITCODE -ne 0) { throw 'Could not verify release assets.' }
$names = @((($release | ConvertFrom-Json).assets) | ForEach-Object { $_.name })
foreach ($file in $files) {
    if ([IO.Path]::GetFileName($file) -notin $names) { throw "Missing release asset: $file" }
}
gh release edit $Tag --draft=false --latest --notes-file dist/release-notes.md
if ($LASTEXITCODE -ne 0) { throw 'Release publication failed.' }
