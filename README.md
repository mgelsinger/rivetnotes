# Rivet

Rivet is a Windows-native text editor focused on fast startup, clean behavior,
and reliable recovery. It is intentionally compact: the core workflows are
implemented deeply instead of spreading effort across a large plugin surface.

Version 0.4.27 adds font selection, a line-number toggle, date/time insertion,
better save defaults, and explicit portable profiles, with fixes for Recent
Files, window layout, and dialog themes. See the
[releases](https://github.com/mgelsinger/rivetnotes/releases) and
[changelog](CHANGELOG.md) for downloads and version history.

## Why Rivet

- Native Win32 UI with a Scintilla editing engine
- Strong session recovery model with periodic snapshots and crash-safe writes
- Fast, predictable keyboard-driven editing flow
- Minimal visual noise with a practical status bar and focused menus
- Spelling underlines for notes and Markdown, using Windows dictionaries
- Optional background updates with controls in the existing status bar

## Core Capabilities

### Editing

- Multi-document tabs with three placements:
  - `Top` (classic horizontal tabs)
  - `Left` (vertical list)
  - `Right` (vertical list)
- Resizable vertical tab panel with persisted width
- Dirty document indicators in both top and vertical tab views
- Word wrap toggle and `Always On Top` toggle in the `View` menu
- `View > Font...` selects the editor's font family and size, remembered across
  sessions. The default remains Consolas 11; zoom still works independently.
- `View > Line Numbers` shows or hides the gutter, remembered across sessions
- `Edit > Insert Date/Time` (`F5`) inserts local time as `YYYY-MM-DD HH:MM`
- Save dialogs start with the current tab name and default to the selected
  language's extension. Auto mode preserves an existing file's extension.
- Zoom (menu, shortcuts, or Ctrl+mousewheel) shared across all tabs and
  persisted between sessions
- Live word count in the status bar
- `Reload from Disk` in the `File` menu (with unsaved-changes confirmation)

### Search and Navigation

- Find/Replace workflow:
  - `Ctrl+F`, `Ctrl+H`
  - `F3`, `Shift+F3`
  - Match case, whole word, regex, wrap
  - `Replace All` grouped into a single undo step
- `Go To Line` (`Ctrl+G`)
- Find in Files with cancellation, validated regexes, and bounded results

### File and Session Safety

- Session restore with open tabs and active tab tracking
- Recent Files (MRU) menu, persisted across sessions
- Encoding detection: UTF-8, UTF-8 BOM, UTF-16 LE/BE, with Windows-1252
  (ANSI) fallback for legacy files
- Periodic backup snapshots for unsaved changes
- Atomic document saves, session writes, and backups, with recovery copies on
  replacement failure
- Dirty-tab close confirmation; final recovery checkpoints on close and Windows
  sign-out/shutdown
- Encoding and embedded NUL preservation through backup and restore
- Stale temp cleanup at startup

### Language and Text Tools

- Syntax highlighting for common formats (JSON, XML, Python, PowerShell,
  YAML, HTML, CSS, C/C++, and **Markdown**)
- Markdown heading folding (code-fence aware)
- Text transforms:
  - `Uppercase`
  - `Lowercase`
  - `Trim Leading + Trailing Whitespace`
- Clipboard path helpers:
  - `Copy Full Path`
  - `Copy Filename`
  - `Copy Directory Path`

### Spellcheck

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

## Keyboard Shortcuts

| Action | Shortcut |
|---|---|
| New file | `Ctrl+N` |
| Open | `Ctrl+O` |
| Save | `Ctrl+S` |
| Save all | `Ctrl+Shift+S` |
| Close tab | `Ctrl+W` |
| Cycle tab placement | `Ctrl+Alt+T` |
| Find | `Ctrl+F` |
| Replace | `Ctrl+H` |
| Insert date/time | `F5` |
| Find next / previous | `F3` / `Shift+F3` |
| Go to line | `Ctrl+G` |
| Uppercase / Lowercase | `Ctrl+Shift+U` / `Ctrl+U` |
| Zoom in / out / reset | `Ctrl+=` / `Ctrl+-` / `Ctrl+0` |

## Installation

- [Download an installer or portable build](https://github.com/mgelsinger/rivetnotes/releases/latest).
- Installer: `rivet-<version>-setup.exe`, with per-user or system-wide installation.
- Portable: `rivet-<version>-win64-portable.zip`.
- If SmartScreen warns, use `More info` then `Run anyway`

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

Maintainers: see [the updater plan](docs/QUIET-UPDATER-PLAN.md) and
[release instructions](docs/UPDATER-RELEASES.md).

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

## Build From Source

### Requirements

- Windows 11 x64
- Rust stable toolchain
- Visual Studio C++ build tools and a Windows SDK for the native editor libraries
- Inno Setup to build or test the installer

### Commands

```powershell
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo run
```

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

## Project Quality

- CI enforces formatting, linting, tests, and dependency audits
- Unit tests cover core session, settings, text transform, and command behavior
- Build metadata is embedded into `Help -> About Rivet`
- Native data-safety regressions run in CI using hidden windows and an isolated
  profile. The legacy uninstall test confirms that notes survive uninstall.
- Native editing regressions check font and line-number behavior, date insertion,
  save history, deleted-file handling, layout, and dialog backgrounds. CI also
  exercises the actual accelerator table in optimized builds.
- Find in Files displays up to 10,000 matches, skips physical lines over 1 MiB,
  and avoids directory junctions and symbolic links. Narrow the search if a limit
  is reported. Its text decoding currently targets UTF-8 files.
- The [September safety review](docs/SAFETY-REVIEW-2026-09-25.md) records fixes,
  test evidence, and remaining manual checks for version 0.4.25.
- Desktop and updater integration tests can be run with
  `cargo test -- --ignored --test-threads=1`. They require an installed Windows
  spelling provider and internet access, and use a hidden editor with temporary
  user data. Build `cargo build --release --bin rivet` first for the helper test.
  Installer smoke tests also require Inno Setup; run
  `scripts/test-updater-installer.ps1` with `INNO_SETUP_ISCC` set. Add
  `-Scope Machine` from an already elevated test session to check system-wide
  installation. Both scopes must pass before a release is published; interactive
  Windows approval also needs the [manual QA checks](docs/QA-CHECKLIST.md).

## Contributing

See `CONTRIBUTING.md` for contribution rules and workflow.

## License

MIT. See `LICENSE`.

Third-party notices: `NOTICE.txt` plus `THIRD_PARTY_NOTICES/NOTICE.txt`,
`THIRD_PARTY_NOTICES/Scintilla.txt`, and `THIRD_PARTY_NOTICES/Lexilla.txt`.
