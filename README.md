# Rivet

Rivet is a Windows-native text editor focused on fast startup, clean behavior,
and reliable recovery. It is intentionally compact: the core workflows are
implemented deeply instead of spreading effort across a large plugin surface.

This README describes the current source. For features available in published
builds, see the [releases](https://github.com/mgelsinger/rivetnotes/releases) and
[changelog](CHANGELOG.md).

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
- Find in Files with cancel support

### File and Session Safety

- Session restore with open tabs and active tab tracking
- Recent Files (MRU) menu, persisted across sessions
- Encoding detection: UTF-8, UTF-8 BOM, UTF-16 LE/BE, with Windows-1252
  (ANSI) fallback for legacy files
- Periodic backup snapshots for unsaved changes
- Crash-safe atomic writes for session and backup data
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

Versions without the updater need one manual installation of an updater-enabled
release first. Maintainers: see [the updater plan](docs/QUIET-UPDATER-PLAN.md) and
[release instructions](docs/UPDATER-RELEASES.md).

## Shell Integration

- Open files from the command line: `rivet.exe <file> [more files]`
- Single instance: files launched while Rivet is running open as tabs in the
  existing window
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

## Project Quality

- CI enforces formatting, linting, and tests
- Unit tests cover core session, settings, text transform, and command behavior
- Build metadata is embedded into `Help -> About Rivet`
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
