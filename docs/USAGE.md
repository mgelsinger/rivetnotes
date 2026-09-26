# Using Rivet

[Back to the README](../README.md)

Details for spelling, updates, file integration, and portable profiles. For the
latest installer or ZIP, visit [Releases](https://github.com/mgelsinger/rivetnotes/releases/latest).

## Spellcheck

- Spelling underlines for untitled notes, plain text, and Markdown prose using
  installed Windows dictionaries, enabled by default. Toggle with `View > Spellcheck`.
  The preference is remembered between sessions. Checking happens after a short
  typing pause, without changing your text or undo history.
- Spelling uses the Windows regional language when supported, with installed
  US/UK English as fallbacks. If none is available, add the language's Basic typing
  feature in Windows Settings and turn Spellcheck off and on again.
- URLs, email addresses, and Markdown code/link destinations are excluded.
  Source files are skipped unless you explicitly choose `View > Language > Plain Text`
  or Markdown. Checks are disabled in Large File Mode and above 2 MiB; Markdown
  physical lines longer than 16 KiB are skipped to keep checks bounded.

Spellcheck provides underlines only. It does not offer replacement suggestions
or change the words you type.

## Optional Updates

Updates are **off by default**. Enable them in Help to check once a day and
download in the background. Progress and the ready action appear in Rivet's
existing status bar, with no separate updater window or installer wizard.

| Installation | Help-menu option | When installation happens |
| --- | --- | --- |
| Per-user installer | `Automatically update on exit` | Quietly after a normal close, leaving Rivet closed. Choose `Restart to update` to reopen afterward. |
| System-wide installer | `Automatically download updates` | Only after `Restart to update (administrator approval)`. Windows may request consent or administrator credentials. |
| Portable or script-installed | Manual updates | Download and replace the installed files yourself. |

Ordinary closing of a system-wide install never requests administrator approval.
If approval is cancelled, Rivet reopens and leaves the update pending. Updates
preserve the existing installation directory and scope.

`Help > Check for updates` checks immediately and reports the result inline.
With automatic updates off, a manual download installs only when you choose
`Restart to update`. Turning the automatic option off cancels pending work.
Normal save checks still apply: cancelling or failing a save postpones the
update. Restarts follow your existing session preferences.

Downloads must match signed release metadata and a verified installer hash.
Rivet never forces a Windows restart. Windows security prompts are controlled
by Windows and can still appear.

Public versions through 0.4.22 need one manual installation of 0.4.23 or later
before they can receive automatic updates. To check an update manually, choose
`Help > Check for updates`, wait for the status bar to report that the update is
ready, and choose `Help > Restart to update`. Confirm the installed version in
`Help > About Rivet` after Rivet reopens.

Maintainers: see [the updater plan](QUIET-UPDATER-PLAN.md) and
[release instructions](UPDATER-RELEASES.md).

## Shell Integration

- Open files from the command line: `rivet.exe <file> [more files]`
- Single instance per profile: files launched while Rivet is running open as
  tabs in that profile's existing window. Separate portable profiles can run
  alongside an installed copy.
- The installer adds an `Open with Rivet` Explorer context menu entry and
  registers Rivet in the `Open with` dialog and
  `Settings > Default apps` so it can be set as the default editor for
  text-like file types; uninstalling removes the registration
- The portable build supports the command line and single-instance behavior
  but does not register context menu or default-app entries

## Data and Configuration

Rivet stores state under `%LOCALAPPDATA%\Rivet` (fallback `%APPDATA%\Rivet`):

- `settings.json` for UI settings, spellcheck, and update preferences
- `sessions\session.json` for remembered documents/session state
- `backup\*.bak` for snapshot files
- `updates\` for verified downloads and updater diagnostics

### Self-contained portable profiles

To keep an extracted copy's settings and recovery data beside the executable,
close that copy and create an empty file named **`rivet-portable`** (no extension)
beside `rivet.exe`. On its next launch, Rivet uses the adjacent `data` directory,
including `data\logs`. Without this marker, existing AppData behavior continues.
An inaccessible portable directory reports an error instead of switching profiles.

The marker selects a separate profile; it does not move existing notes. To carry
an existing profile over, close all Rivet windows, copy the contents of
`%LOCALAPPDATA%\Rivet` into the new copy's `data` directory, then create the marker.
Keep the original copy until you have verified the result, and do not overwrite
another existing portable profile. Separately saved documents still live at their
original paths and must be copied separately if moving to another computer.

Portable profiles use manual executable updates. Keep the `rivet-portable` file
and `data` directory when replacing the executable. Changing the marker takes
effect after that copy is closed and relaunched.

## Search and large files

Find in Files displays up to 10,000 matches, skips physical lines over 1 MiB,
and avoids directory junctions and symbolic links. Narrow the search if a limit
is reported. Its text decoding currently targets UTF-8 files.

Large File Mode starts at a configurable threshold (20 MiB by default). It
disables syntax highlighting and, by default, word wrap and smart highlighting
to keep editing responsive. Spellcheck also skips documents above 2 MiB.
