# Publishing updates

The public key in `assets/update-public-key.hex` is part of Rivet's update trust
root. Keep it stable. The corresponding private key must never be committed,
packaged, printed in logs, or put in a release asset.

## Signing setup

The initial key was generated outside the repository in
`%LOCALAPPDATA%\RivetRelease\update-signing.key`, with access restricted to the
current Windows user and SYSTEM. Back up that file securely. The release
workflow reads its hexadecimal contents from the GitHub Actions repository
secret `RIVET_UPDATE_SIGNING_KEY`. That secret is configured for
`mgelsinger/rivetnotes`; the private key remains outside the workspace.

To provision that secret from the protected file, without printing the key:

```powershell
Get-Content -Raw "$env:LOCALAPPDATA\RivetRelease\update-signing.key" |
    gh secret set RIVET_UPDATE_SIGNING_KEY --repo mgelsinger/rivetnotes
```

`scripts/configure-updates.ps1` is for initializing a new trust root only. It
refuses to overwrite an existing key. Do not generate a new key on another
computer to work around a missing backup. Existing installations trust the
original key; changing it requires a separately planned migration.

## Release procedure

1. Bump the stable Cargo version and lockfile, and add a matching changelog
   section. Existing `v0.4.22` assets must not be replaced with new bytes.
2. Run formatting, all-target Clippy, unit and native integration tests. Build
   the portable package and installer. The Inno AppId intentionally preserves
   its historical extra closing brace for upgrade compatibility.
3. Run `scripts/test-updater-installer.ps1` with `INNO_SETUP_ISCC` set. It compiles
   isolated installers with unique identities, no shell registration or icons,
   and verifies silent install/upgrade, payload replacement, no relaunch,
   read-only installer locking, and refusal while app/setup mutexes exist. Repeat
   with `-Scope Machine` in an already elevated test session to verify shared
   installation and HKLM registration. The script never requests elevation. Logs
   remain under `target/updater-smoke-*`; it does not use the real Rivet install.
4. Create/push the matching version tag when ready to publish, or dispatch the
   release workflow with that existing tag. It checks out that exact tag and
   checks its version before building.
5. The workflow signs and independently verifies `update.json`, hashes the
   installer, ZIP, and manifest, and uploads all assets to a draft release.
   Only a complete upload is published. A failed draft can be retried; a
   published release is immutable to this workflow.

For a local signing check, set `RIVET_UPDATE_SIGNING_KEY` in the process
environment from the protected file and run `scripts/prepare-update.ps1 -Tag
vX.Y.Z`. Clear that environment variable afterward. This does not publish.

The feed is the `update.json` asset on the latest stable release. The signed
payload contains version, platform, size, and SHA-256; the signature covers its
exact UTF-8 bytes. The client derives the installer URL from the verified
version and accepts only newer stable x64 releases up to 128 MiB.

## Bootstrap and limitations

Existing users need one normal installation of the first updater-enabled
release. Updates remain off until enabled in Help. Portable and script-installed
copies remain manual. Metadata signing authenticates Rivet's
update feed; it is separate from Windows Authenticode code signing.

For per-user installs, normal close installs without relaunching. An explicit restart uses the user's
existing session preferences. Backup or session-save failures postpone the
update. Ordinary unsaved-document prompts remain part of normal shutdown.
Automatic network failures stay in the log; manual failures appear in Rivet's
status bar. Helper failures are recorded in `updates/result.json` and reported
inline at the next launch; `updates/install.log` contains per-user installer diagnostics.

System-wide installations download in the background but require the explicit
`Restart to update (administrator approval)` action. Windows can request consent
or another administrator's credentials. The helper elevates only the verified
installer, with `/ALLUSERS` and the registered directory; the editor and helper
retain the original user's profile and privileges. Cancelled approval leaves
the staged update intact and reopens Rivet with an inline postponement message.
Shared installer logs use Inno's unique `Setup Log ...` file in the elevated
account's temporary directory. No Windows reboot is forced.

Before release, manually verify UAC acceptance and cancellation, alternate-admin
credentials, and refusal while another user's editor holds installation files.
The automated machine smoke test requires an already elevated runner and does
not exercise the interactive Windows consent screen.

Inno's very-silent mode suppresses its normal installer UI. Windows security
software and errors before the installer parses arguments are outside Rivet's
control. Normal closing never requests elevation; shared updates require the
explicit action described above. See Microsoft's
[ShellExecuteEx documentation](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/ns-shellapi-shellexecuteinfow)
and Inno's [installer arguments](https://jrsoftware.org/ishelp/topic_setupcmdline.htm).
