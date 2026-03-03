# Changelog

All notable changes to this project will be documented in this file.
The format is based on Keep a Changelog, and this project adheres to SemVer.

## [Unreleased]

- TBD.

## [0.4.3] - 2026-03-03

- Added a `View -> Dark Mode` toggle so users can switch between light and dark themes.
- Persisted editor theme choice in `settings.json` via a new `editor_dark` field.
- Fixed startup theme initialization to honor persisted settings instead of forcing dark mode.

## [0.4.2] - 2026-03-03

- Added selection-driven Smart Highlight using Scintilla container indicators
  (`INDIC_ROUNDBOX`) with theme-aware colors/alpha and bounded
  `SCI_SEARCHINTARGET` scanning.
- Added temporary line folding commands in `View`: `Hide Lines` and
  `Unhide All Lines`, including keyboard shortcuts (`Alt+H`, `Alt+Shift+H`).
- Added document-tab keyboard cycling with wrap-around for
  `Ctrl+Tab` / `Ctrl+Shift+Tab` and `Ctrl+PageDown` / `Ctrl+PageUp`.
- Introduced Large File Mode restrictions with configurable threshold and
  toggles in `settings.json`, including smart-highlight suppression by default
  and optional global word-wrap deactivation.
- Updated status/title indicators to surface Large File Mode state and
  smart-highlight truncation ("Too many matches").
- Added unit tests for new settings fields/clamping and large-file/token helper logic.

## [0.4.1] - 2026-03-03

- Fixed CI failures for `cargo fmt --check` and `cargo clippy -- -D warnings`
  on the `v0.4.0` line.
- Aligned release gating with CI by adding `fmt` and `clippy` checks to
  `.github/workflows/release.yml` before tests/build/publish.
- Validated the updated CI pipeline end-to-end on `main` with all required jobs green.

## [0.4.0] - 2026-03-03

- Introduced a `TabStripHost` architecture that supports three tab placements:
  `Top`, `Left`, and `Right`, while keeping document logic unchanged.
- Added persisted UI settings in `%LOCALAPPDATA%\Rivet\settings.json`:
  `tab_placement` (`top|left|right`) and `vertical_tab_width_px`.
- Replaced vertical `ListBox` tabs with a custom-drawn `ListView`-based vertical
  tab panel to avoid unsupported Win32 `TCS_VERTICAL` behavior under ComCtl32 v6.
- Implemented vertical tab theming via `NM_CUSTOMDRAW` with explicit light/dark
  palette colors for background, selection, hover, and text.
- Added/updated `View -> Tabs -> Top|Left|Right` menu controls with checked
  radio-style behavior and persistent placement updates.
- Kept `Ctrl+Alt+T` placement cycling and wired it through the new placement model.
- Implemented splitter drag resize with capture-based behavior and persisted width.
- Switched child-window layout positioning to `SetWindowPos` for tabs, splitter,
  status bar, and editor windows.
- Added placement-agnostic tab context hit testing for both top `TabCtrl` and
  vertical `ListView` tabs.
- Standardized dirty tab label rendering in both tab modes with a trailing `*`.
- Added targeted settings tests for serialization shape, defaults, roundtrip,
  and width clamping.

## [0.3.1] - 2026-03-02

- Added `Edit -> Go To Line...` with `Ctrl+G` and Scintilla `SCI_GOTOLINE` navigation,
  including 1-based line input prefilled from the current caret line and clamped to file bounds.
- Completed core Find/Replace behavior with standard keyflow:
  `Ctrl+F`, `Ctrl+H`, `F3`, `Shift+F3`, wrap-around, match case, whole word,
  and `Replace` now advancing to the next match after replacement.
- Kept `Replace All` as a single undo step via grouped Scintilla undo actions.
- Fixed CI clippy gating issue (`collapsible_if`) so `cargo clippy -- -D warnings`
  passes in GitHub Actions.

## [0.3.0] - 2026-03-01

- Implemented Notepad++-style `remember_session` + `session_snapshot_periodic_backup`
  behavior with default-on periodic backups and no save prompts on exit when enabled.
- Added crash-resilient atomic writes for backup and session files
  (`ReplaceFileW` with `MoveFileExW` fallback), plus startup cleanup for stale temp files.
- Implemented backup-first restore semantics for dirty tabs at shutdown and full-tab
  session restoration (named and untitled documents).
- Added global `View` menu with checkable toggles for `Word Wrap` and `Always On Top`,
  with persisted settings and startup re-application.
- Upgraded find/replace internals to `SCI_SEARCHINTARGET`-based search with grouped
  `Replace All` undo behavior and Notepad++-style replace flow.
- Updated status bar fields to show authoritative editor state:
  `Ln/Col`, `Sel`, `EOL`, `ENC`, and dirty indicator.
- Added `Help -> About Rivet` modal with version, git SHA, build UTC, source URL,
  and local data directory, including copy-to-clipboard action.
- Added build metadata injection in `build.rs`
  (`RIVET_VERSION`, `RIVET_GIT_SHA`, `RIVET_BUILD_UTC`, `RIVET_SOURCE_URL`).
- Hardened CI with separate `fmt`, `clippy`, `test`, and scheduled RustSec `cargo audit`
  workflow jobs.
- Added release compliance assets:
  `NOTICE.txt` and `THIRD_PARTY_NOTICES/Scintilla-Lexilla-License.txt`,
  and included them in portable + installer packaging.

## [0.2.1] - 2026-03-01

- Added a tab-bar right-click context menu with tab-scoped actions:
  `Save`, `Save As...`, `Duplicate Tab`, `Close`, `Close Others`,
  `Close Tabs to the Left`, and `Close Tabs to the Right`.
- Implemented tab hit-testing on right click and selection handoff so actions
  apply to the clicked tab.
- Expanded the editor right-click context menu with standard commands:
  `Undo`, `Redo`, `Cut`, `Copy`, `Paste`, `Delete`, and `Select All`,
  while keeping text transform and trim actions available.
- Added command enable/disable logic for editor context actions using Scintilla
  capability queries (`SCI_CANUNDO`, `SCI_CANREDO`, `SCI_CANPASTE`,
  and selection-state checks).
- Added Scintilla wrapper functions/constants needed for context-menu command
  state and delete behavior.

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
