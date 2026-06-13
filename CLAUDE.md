# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```powershell
cargo fmt --check          # check formatting (CI gate)
cargo fmt                  # auto-format
cargo clippy -- -D warnings  # lint (warnings are errors)
cargo test                 # run all tests
cargo test <module::path>  # run a single test, e.g. cargo test session::tests::test_decide_restore_source
cargo run                  # debug build + launch
cargo build --release      # release binary

# Packaging (Windows only)
.\scripts\package.ps1       # produces dist/*.zip portable build
.\scripts\build-installer.ps1  # produces dist/*.exe installer (requires Inno Setup)

# Verbose logging at runtime
$env:RIVET_VERBOSE = "1"; cargo run
```

CI runs on Windows (windows-latest) and requires fmt + clippy + test to pass. A nightly RustSec audit also runs.

## Architecture Overview

Rivetnotes is a **Windows-native text editor** written in Rust. All unsafe Win32/FFI code is isolated in `src/platform/win32.rs`; the rest is safe Rust.

### Module Responsibilities

| Module | Role |
|--------|------|
| `platform/win32.rs` (~7200 lines) | Win32 message loop, all UI, menus, dialogs, tab control, status bar |
| `editor/scintilla.rs` | Scintilla C++ library bindings — communicates via Windows messages to embedded child window |
| `editor/markdown.rs` | Pure (testable) markdown heading fold-level computation, code-fence aware |
| `app/document.rs` | Document metadata: path, encoding, EOL mode, dirty flag, cursor position, backup path, large-file flag |
| `app/session.rs` | `SessionData` (open tabs, active tab, schema version), restore logic, periodic checkpoint |
| `app/settings.rs` | `UiSettings`: tab placement (Top/Left/Right), vertical-tab width, dark mode, smart-highlight, large-file thresholds, recent files (MRU) |
| `storage/atomic_write.rs` | Crash-safe atomic writes (temp-file + replace), JSON serialization, stale-temp cleanup |
| `textops/` | Text transforms: trim whitespace, strikethrough |
| `commands/` | Clipboard helpers (copy path/filename/directory), selection case checks |
| `logging.rs` | Rotating file logs to `%APPDATA%\Rivet\logs\`, controlled by `RIVET_VERBOSE` env var |
| `error.rs` | Central `AppError` type; user-facing errors shown via `platform::win32::show_error()` |

### Data Flow

1. `main.rs` → `platform::win32::run()` (blocking Win32 message loop)
2. Win32 events → command IDs → `Document`/`SessionData` state updates
3. Session checkpoint every ~7 seconds via timer message
4. Dirty documents get `.bak` backup written atomically
5. On next launch: `decide_restore_source()` picks Backup → Disk → Skip

### Vendored C++ (third_party/)

- **Scintilla**: editing engine, compiled as static lib via `build.rs` (C++17, MSVC `/EHsc /utf-8`)
- **Lexilla**: syntax highlighting lexers (JSON, XML, Python, PowerShell, YAML, HTML, CSS, C/C++, Markdown); auto-disabled in Large File Mode. Markdown also gets heading folds computed in `editor/markdown.rs` (the lexer itself has no folder) and applied on a debounced timer.

### Session & Data Storage

Files live under `%LOCALAPPDATA%\Rivet\` (fallback `%APPDATA%\Rivet\`):
- `settings.json` — UI settings
- `sessions/session.json` — open tabs + session state (schema v2)
- `backup/*.bak` — document snapshots

### Key Patterns

- **Clipboard trait**: `WinClipboard` (production) / `TestClipboard` (tests) — keeps `unsafe` out of unit tests.
- **Large File Mode**: triggered at configurable threshold (default 20 MB); disables word wrap and syntax highlighting.
- **Encodings**: UTF-8, UTF-8 BOM, UTF-16 LE, UTF-16 BE detected on load and preserved on save. Files that are neither valid UTF-8 nor BOM-tagged fall back to Windows-1252 (ANSI); the status bar reflects the document's encoding. Saving as ANSI errors on characters CP1252 cannot represent (use Save As, which writes UTF-8).
- **EOL**: CRLF / LF detected per-document and preserved.
- **Tab placement**: cyclic enum Top → Left → Right → Top; Left/Right render a resizable vertical tab strip.
