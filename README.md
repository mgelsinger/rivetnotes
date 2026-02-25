# Rivet

Rivet is a Windows-native text editor built for speed and calm. It keeps the
feature set intentionally small and makes those essentials feel polished and
predictable. If you want a clean editor that starts fast and stays out of your
way, Rivet is the point.

## Features

- Tabbed editing with horizontal or vertical tab layout
- Session restore (reopens files and caret position)
- Find/replace plus Find in Files with cancellation
- Syntax highlighting for common formats
- Word wrap toggle (on by default)
- Status bar with encoding, line endings, line/column, and word count
- Always-on-top toggle for quick note-taking

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

## License

MIT. See `LICENSE`.
