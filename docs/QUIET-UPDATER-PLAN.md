# Quiet optional updater

## User experience

- Off by default. A Help-menu checkbox enables background downloads. Per-user installations apply on normal exit; shared installations wait for an explicit restart with administrator approval.
- Check after startup settles and then at most once per day while running. A manual Help-menu check reports its result inline.
- Use Rivet's existing status bar for checking, download progress, errors from manual checks, and "Update ready - click to restart". No updater dialogs.
- For per-user installs, normal exit installs without reopening Rivet. Clicking the ready indicator saves/closes normally, updates, and reopens Rivet. Cancelled shutdown or failed saves postpone installation.
- Preserve the existing install directory and scope, session preferences, and user data.
- System-wide copies expose `Restart to update (administrator approval)`. Elevate only the verified installer, keep the helper in the original user account, and handle cancelled UAC as postponement. Normal close never requests elevation.
- Portable and script-installed copies remain manual.

## Implementation

1. Add a signed JSON release protocol: Ed25519-signed version, architecture, installer size, and SHA-256 hash. Embed the public key, limit accepted versions and sizes, and derive download URLs from the verified version.
2. Use native WinHTTP on one cancellable worker. Require HTTPS, bounded responses, timeouts, and secure redirects. Stage only verified installers under Rivet's user-data update cache; revalidate cached downloads before reuse.
3. Persist opt-in and last-check time. Detect installed copies by matching the current executable directory to Rivet's existing Inno uninstall registration. Retain the historical AppId exactly for upgrade compatibility.
4. Add status-bar progress and a restart action. Automatic network failures are logged quietly; manual failures remain inline. Turning the option off cancels work and prevents installation.
5. Launch a copy of Rivet as a hidden update helper only after successful shutdown preparation. It waits for the original process to exit, verifies and locks the installer again, invokes Inno in very-silent/no-reboot/no-force-close mode, records the result, and relaunches only for an explicit restart request.
6. Add a release signing tool and protected local key provisioning. Extend the existing release workflow to check out the requested tag, validate its version, sign final artifacts, and upload metadata with the installer before publication. Never store the private key in Git.
7. Add protocol, scheduling/cancellation, helper, UI, and installer tests. Run formatting, all-target linting, all tests, and an optimized build.

## Delivery and validation

Existing users need one ordinary installation of the first updater-enabled release. Publishing that release is a separate action from implementing and testing this feature. The stable feed is the signed `update.json` asset on the latest GitHub release.

Test modified manifests and downloads, wrong keys, downgrades/prereleases, oversized/truncated responses, cancellation, offline/manual checks, cached readiness, matching/mismatching install paths, cancelled shutdown, silent installer flags, and update failures. Use isolated test directories and installer identities so validation does not replace the user's installed Rivet.

## Implementation status

Implemented for v0.4.23. Formatting, all-target Clippy, 131 regular
tests, five explicitly run native/network/helper tests, and the optimized build
pass. Production-key signing and tampered-payload rejection also pass. The
GitHub repository signing secret is configured. The v0.4.23 release workflow
publishes the first stable update feed after both installer scopes pass.

The per-user installer smoke test passes locally with the existing per-user
Inno Setup compiler. It verifies silent installation and upgrade, file locking,
no automatic relaunch, and the app/setup mutex protections. Both installer scopes
are required by the release workflow; the system-wide test still needs an
elevated session. Interactive UAC acceptance, cancellation, and alternate-admin
credentials also need a Windows manual check.
See `UPDATER-RELEASES.md` for
the bootstrap and key-backup procedure.
