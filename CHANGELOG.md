# Changelog

All notable changes to this project will be documented in this file.
The format is based on Keep a Changelog, and this project adheres to SemVer.

## [Unreleased]

- TBD.

## [0.2.0] - 2026-03-01

- Added parent-owned editor context menu with exactly three commands:
  `Uppercase`, `Lowercase`, and `Trim Leading + Trailing Whitespace`.
- Disabled Scintilla default popup (`SCI_USEPOPUP(SC_POPUP_NEVER)`) so context
  menu behavior is consistent and app-controlled.
- Added Scintilla key bindings for text transforms:
  `Ctrl+U` (lowercase) and `Ctrl+Shift+U` (uppercase).
- Added `Edit -> Copy to Clipboard` operations:
  `Copy Full Path`, `Copy Filename`, and `Copy Directory Path`.
- Added enable/disable command state logic so no-op actions are greyed out:
  selection-based transform enablement and saved-path-based copy enablement.
- Added pure command/text logic modules and unit tests for trim semantics,
  copy-path behavior, and command enablement decisions.
- Added/updated CI to enforce `cargo fmt --check`,
  `cargo clippy -- -D warnings`, and `cargo test` on Windows.

## [0.1.2] - 2026-02-25

- Removed the editor's left gutter/padding for a flush text area.
- Improved dark-mode caret visibility.
- Focus editor automatically when selecting tabs.
- Suppressed the console window for release builds.

## [0.1.1] - 2026-02-25

- Vertical tab layout with resizable sidebar and layout cycling.
- Status bar enhancements with line/column and word count.
- Word wrap enabled by default with a toggle.
- Added always-on-top toggle and new file/save all commands.
- Multi-size app icon embedded and installer polish.
- Added unit tests for core text/session/find logic and CI release size reporting.

## [0.1.0] - 2026-02-24

- Win32 scaffolding with Scintilla editor host.
- File I/O with encoding and EOL preservation.
- Tabs, session restore, and edit commands.
- Find/replace and find-in-files with cancellation.
- Lexilla-backed syntax highlighting for a curated set.
- Editor dark mode toggle and per-monitor DPI awareness v2.
- Local logging with rotation and opt-in verbosity.
