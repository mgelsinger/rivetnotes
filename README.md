# Rivet

Rivet is a Windows-native text editor built for speed and calm. It keeps the
feature set intentionally small and makes those essentials feel polished and
predictable. If you want a clean editor that starts fast and stays out of your
way, Rivet is the point.

## Features

- Tabbed editing with horizontal or vertical tab layout
- Session snapshot + periodic backup (Notepad++-style restore of unsaved work)
- Find/Replace core flow: `Ctrl+F`, `Ctrl+H`, `F3`, `Shift+F3`, `Replace`, `Replace All` (single undo step), wrap-around, match case, whole word
- Go To Line (`Ctrl+G`) using Scintilla line navigation
- Find in Files with cancellation
- Syntax highlighting for common formats
- Word wrap toggle (on by default) and `Always On Top` toggle in `View`
- Status bar truth fields: `Ln/Col`, `Sel`, `EOL`, `ENC`, dirty indicator
- Crash-safe atomic writes for session/backup files with stale temp cleanup on startup
- About dialog with build metadata (version, commit SHA, build UTC, source, data dir)
- Minimal menu bar (`File`, `Edit`, `View`, `Help`) and tiny editor context menu
- Text transforms: `Uppercase`, `Lowercase`, `Trim Leading + Trailing Whitespace`
- Shortcut support for transforms (`Ctrl+U`, `Ctrl+Shift+U`) and tab close (`Ctrl+W`)
- Clipboard path helpers in `Edit -> Copy to Clipboard`:
  - `Copy Full Path`
  - `Copy Filename`
  - `Copy Directory Path`

## Install

- Installer: download `rivet-<version>-setup.exe` from Releases
- Portable: download `rivet-<version>-win64-portable.zip` from Releases
- Windows SmartScreen: if prompted, click `More info` → `Run anyway`.

## Build it from source

Prerequisites:

- Rust toolchain (stable)
- Windows 11 x64

Commands:

```powershell
cargo fmt
cargo clippy -D warnings
cargo test
cargo run
```

## FAQ

- Why no plugins? Rivet is intentionally small to stay fast and predictable.
- Is there an auto-updater? Not in the MVP; updates are manual to keep the attack surface small while considering avenues of distribution.
- Does it support large files? Yes, with a Large File Mode that disables heavy features.
- Where are settings stored? Local config files under the user's profile.
- Is there a portable build? Yes, a portable zip is provided alongside the installer.

## Contributing

See `CONTRIBUTING.md`. At minimum, CI must be green before merging.
CI runs `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`,
and a scheduled RustSec dependency audit.

## License

MIT. See `LICENSE`.
Third-party notices are in `NOTICE.txt` and `THIRD_PARTY_NOTICES/`.
