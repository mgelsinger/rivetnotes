param(
    [string]$KeyDirectory = (Join-Path $env:LOCALAPPDATA 'RivetRelease')
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$keyFile = Join-Path $KeyDirectory 'update-signing.key'
$publicFile = Join-Path $repo 'assets\update-public-key.hex'
if (Test-Path -LiteralPath $publicFile) {
    throw 'The project already has an update public key. Restore its matching private key instead of generating a replacement.'
}
if (Test-Path -LiteralPath $keyFile) {
    throw "A signing key already exists at $keyFile. Do not replace a key used by published releases."
}
New-Item -ItemType Directory -Path $KeyDirectory -Force | Out-Null
$acl = New-Object System.Security.AccessControl.DirectorySecurity
$acl.SetAccessRuleProtection($true, $false)
$user = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
foreach ($sid in @($user, [System.Security.Principal.SecurityIdentifier]::new('S-1-5-18'))) {
    $rule = [System.Security.AccessControl.FileSystemAccessRule]::new($sid, 'FullControl', 'ContainerInherit,ObjectInherit', 'None', 'Allow')
    $acl.AddAccessRule($rule)
}
Set-Acl -LiteralPath $KeyDirectory -AclObject $acl
Push-Location $repo
try {
    cargo run --bin update-release -- keygen $keyFile $publicFile
    if ($LASTEXITCODE -ne 0) { throw 'Signing key generation failed.' }
} finally { Pop-Location }
Write-Host "Private signing key: $keyFile"
Write-Host 'Keep a secure backup. Configure the RIVET_UPDATE_SIGNING_KEY release secret from this file.'
