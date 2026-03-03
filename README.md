# Rivet

Rivet is a Windows-native text editor focused on fast startup, clean behavior,
and reliable recovery. It is intentionally compact: the core workflows are
implemented deeply instead of spreading effort across a large plugin surface.

## Why Rivet

- Native Win32 UI with a Scintilla editing engine
- Strong session recovery model with periodic snapshots and crash-safe writes
- Fast, predictable keyboard-driven editing flow
- Minimal visual noise with a practical status bar and focused menus

## Core Capabilities

### Editing

- Multi-document tabs with three placements:
  - `Top` (classic horizontal tabs)
  - `Left` (vertical list)
  - `Right` (vertical list)
- Resizable vertical tab panel with persisted width
- Dirty document indicators in both top and vertical tab views
- Word wrap toggle and `Always On Top` toggle in the `View` menu

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
- Periodic backup snapshots for unsaved changes
- Crash-safe atomic writes for session and backup data
- Stale temp cleanup at startup

### Language and Text Tools

- Syntax highlighting for common formats
- Text transforms:
  - `Uppercase`
  - `Lowercase`
  - `Trim Leading + Trailing Whitespace`
- Clipboard path helpers:
  - `Copy Full Path`
  - `Copy Filename`
  - `Copy Directory Path`

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

## Installation

- Installer: download `rivet-<version>-setup.exe` from GitHub Releases
- Portable: download `rivet-<version>-win64-portable.zip` from GitHub Releases
- If SmartScreen warns, use `More info` then `Run anyway`

## Build From Source

### Requirements

- Windows 11 x64
- Rust stable toolchain

### Commands

```powershell
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo run
```

## Data and Configuration

Rivet stores state under `%LOCALAPPDATA%\Rivet` (fallback `%APPDATA%\Rivet`):

- `settings.json` for UI settings such as tab placement and vertical tab width
- `sessions\session.json` for remembered documents/session state
- `backup\*.bak` for snapshot files

## Project Quality

- CI enforces formatting, linting, and tests
- Unit tests cover core session, settings, text transform, and command behavior
- Build metadata is embedded into `Help -> About Rivet`

## Contributing

See `CONTRIBUTING.md` for contribution rules and workflow.

## License

MIT. See `LICENSE`.

Third-party notices: `NOTICE.txt` and `THIRD_PARTY_NOTICES/`.
