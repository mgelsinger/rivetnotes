# Changelog

All notable changes to this project will be documented in this file.
The format is based on Keep a Changelog, and this project adheres to SemVer.

## [Unreleased]

- TBD.

## [0.4.18] - 2026-06-13

### Markdown & syntax

- **Language menu (manual syntax override).** A new **View → Language**
  submenu lets you force a lexer per tab — including on an unsaved **Untitled**
  buffer — so Markdown highlighting (bold `#` headings, `**bold**`, etc.) no
  longer requires saving the file as `.md` first. *Auto (by extension)* remains
  the default and is restorable at any time; the active tab's language is
  check-marked. Large File Mode still suppresses highlighting.

### Editing

- **Enter on a collapsed block no longer types into hidden text.** When the
  caret is on a collapsed user-collapse header, pressing Enter now inserts a
  new **visible** line *below* the whole collapsed region (caret placed there)
  and leaves the collapse intact, instead of inserting an invisible line inside
  the hidden range. Plain Enter only; Ctrl/Alt+Enter and multi-line selections
  are untouched.

### Known issues

- Markdown ATX (`#`) heading highlighting still looks slightly off and needs
  style refinement.
- In dark mode the selected tab renders blue rather than a dark-theme color.

## [0.4.17] - 2026-06-12

### Markdown (priority)

- **Markdown files are now syntax-highlighted.** The vendored Lexilla
  `LexMarkdown` lexer was present but neither compiled nor registered;
  `LexerKind::Markdown` mapped to the `null` lexer, so `.md`/`.markdown`
  files rendered as plain text. Added `LexMarkdown.cxx` to `build.rs`,
  registered `lmMarkdown` in `LexillaMinimal.cxx`, mapped the lexer name to
  `"markdown"`, and added light/dark styles for headings (bold), strong/
  emphasis, lists, blockquotes, links (underlined), strikeout, and code
  spans/blocks.
- **Heading folds are now correct and cheap.** Fold-level computation moved
  to a pure, unit-tested module (`editor/markdown.rs`) that:
  - ignores `#` lines inside fenced (` ``` ` / `~~~`) and indented code
    blocks, so comments-in-code no longer create bogus fold points;
  - enforces the CommonMark "at most 3 leading spaces" rule for ATX
    headings and accepts a trailing `\r`;
  - is applied on a **debounced timer** instead of on every keystroke, and
    only writes the lines whose fold level actually changed. The previous
    code walked and rewrote the whole document on each edit.

### Editing & files

- **Legacy ANSI files open instead of erroring.** Files that are neither
  valid UTF-8 nor BOM-tagged UTF-16 now decode as Windows-1252 (`ANSI`)
  rather than failing with "Unsupported encoding". `Ctrl+S` preserves the
  ANSI encoding; saving a character CP1252 cannot represent reports a clear
  error pointing at `Save As` (which writes UTF-8). The full 0x00–0xFF byte
  range round-trips.
- **The status bar now reports the document's real encoding.** It previously
  read Scintilla's buffer codepage, which is always UTF-8 — so UTF-16 files
  were mislabeled "UTF-8". It now shows UTF-8 / UTF-8 BOM / UTF-16 LE /
  UTF-16 BE / ANSI from the document itself.

### UI

- **New `File → Recent Files` menu (MRU).** Up to 10 most-recently-opened
  paths, newest first, persisted in `settings.json`, with numeric
  accelerators and a `Clear Recent Files` item. Stale entries are pruned
  (with a notice) when clicked. Populated from the Open dialog, drag-drop,
  and Find-in-Files hits — not from session restore.
- **Middle-click now closes vertical tabs too.** The middle-click-to-close
  intercept previously fired only on the Top tab strip; it now works on the
  Left/Right vertical tab strip as well, sharing the same hit-test path as
  the close `×`.

## [0.4.16] - 2026-05-15

- Killed the click-flash on the Left / Right vertical tab strip. When you
  clicked a tab in dark mode, comctl32 briefly painted the light Explorer
  selection rectangle before our `NM_CUSTOMDRAW` background overwrote it
  on the next frame. Fixed by calling `SetWindowTheme(vertical_tabs,
  "DarkMode_Explorer" | "Explorer", NULL)` from `update_tab_host_theme`,
  which swaps the comctl32-drawn chrome to the dark variant instead of
  trying to suppress it. (The `SetWindowTheme(hwnd, "", "")` strip used on
  the top tab control would have re-broken visibility on the ListView, so
  the named-theme path is mandatory here.)
- New helper `dark_mode::apply_explorer_theme(hwnd, dark)` wraps the
  light/dark `Explorer` theme selection.
- Refreshed the docstring on `dark_mode::disable_visual_styles` to describe
  the v0.4.12 `WM_PAINT` reason rather than the abandoned `NM_CUSTOMDRAW`
  story, and added a "do not use on a `LVS_REPORT` ListView" warning so
  future-me doesn't repeat the v0.4.13 mistake.
- Added `docs/QA-CHECKLIST.md` — a permanent manual-smoke list (top tabs,
  vertical tabs, dark mode toggle, session restore, large file mode, etc.)
  so QA before tagging doesn't depend on skimming prior release notes.

### Tab + dark-mode hardening summary (v0.4.9 – v0.4.15)

The patch releases from v0.4.9 through v0.4.15 collectively rebuilt how
the tab strip renders:

- **Top tabs** were switched off `TCS_OWNERDRAWFIXED` so Windows
  auto-sizes each tab to its own filename. The custom paint moved from
  `WM_DRAWITEM` to a full `WM_PAINT` subclass (after `NM_CUSTOMDRAW`
  proved unreliable for `SysTabControl32`), with visual styles stripped
  via `SetWindowTheme`.
- **Vertical tabs** kept their `NM_CUSTOMDRAW` path but had two bugs
  fixed: (1) "which row is active?" now consults `state.active` instead
  of the unreliable `nmcd.uItemState & CDIS_SELECTED`; (2) the click-flash
  in v0.4.16 above.
- Several false starts along the way (notably v0.4.13/v0.4.14 making the
  vertical strip invisible by stripping visual styles on a ListView) are
  preserved as individual entries for archaeology.

If you want the executive summary, this is it. Individual entries follow.

## [0.4.15] - 2026-05-15

- Restored visibility of the Left / Right vertical tab strip (broken in
  v0.4.13, still broken after the v0.4.14 partial revert). Rolled
  `handle_vertical_tab_custom_draw` back to the v0.4.12 painting model
  (`CDRF_NEWFONT | CDRF_NOTIFYPOSTPAINT` at `CDDS_ITEMPREPAINT`, close glyph
  in `CDDS_ITEMPOSTPAINT`) — letting comctl32 draw the row text again is
  what brings the rows back. The v0.4.13 attempt to fully self-paint and
  return `CDRF_SKIPDEFAULT` reliably produces a blank ListView on the
  `LVS_REPORT + LVS_NOCOLUMNHEADER + LVS_EX_FULLROWSELECT` configuration.
- Fixed the "all tabs appear highlighted" bug at the same time by replacing
  `NMLVCUSTOMDRAW.nmcd.uItemState & CDIS_SELECTED` (which is unreliable for
  ListView item state — `CDIS_SELECTED = 1` collides with `LVIS_FOCUSED = 1`)
  with a direct `doc_index == state.active` test. The active doc index is
  the authoritative source of "which tab is selected" used everywhere else
  in the codebase, so only one row paints with `selection_bg` now.

## [0.4.14] - 2026-05-15

- Fixed the vertical tab strip (Left / Right placement) rendering completely
  invisible after v0.4.13. v0.4.13 called `dark_mode::disable_visual_styles`
  on the `WC_LISTVIEWW` to match the v0.4.11 trick used on `WC_TABCONTROLW`,
  but ListViews in `LVS_REPORT` mode rely on visual styles for item layout
  and stripping them collapses row heights so nothing draws. Reverted just
  that one call. The v0.4.13 self-contained `CDDS_ITEMPREPAINT` paint with
  `CDRF_SKIPDEFAULT` is kept — that's what actually stops comctl32 from
  drawing themed selection chrome over our paint, and it works correctly
  with visual styles left on for ListViews.

## [0.4.13] - 2026-05-15

- Fixed the **Left / Right** vertical tab strip ignoring dark mode: every row
  rendered with the bright `selection_bg` color (so all open files looked
  selected) and clicking briefly flashed the light-theme Explorer selection
  rectangle on top of the custom paint. Root cause was the same as the v0.4.12
  top-tab fix — comctl32's themed paint was still running on top of our
  `NM_CUSTOMDRAW` background fill, because the ListView never had visual
  styles stripped and the handler returned `CDRF_NEWFONT | CDRF_NOTIFYPOSTPAINT`
  (which leaves comctl32 in charge of the text and selection chrome).
- `dark_mode::disable_visual_styles(vertical_tabs)` is now called right after
  the ListView is created, and `handle_vertical_tab_custom_draw` now owns the
  full item paint at `CDDS_ITEMPREPAINT` — background, label via `DrawTextW`,
  and the close `×` glyph — then returns `CDRF_SKIPDEFAULT` so comctl32 paints
  nothing further. The `CDDS_ITEMPOSTPAINT` arm is removed.

## [0.4.12] - 2026-05-15

- Fully took over top tab painting from `WM_PAINT` in the existing
  `top_tabs_subclass_proc` instead of trying to coax `NM_CUSTOMDRAW` into
  rendering on a `SysTabControl32`. v0.4.11 stripped visual styles, but
  comctl32 still never delivered useful `CDDS_ITEMPREPAINT` notifications,
  so the strip rendered with the default light theme and the close `×` was
  invisible. The new `paint_top_tab_strip` walks tabs via `TCM_GETITEMCOUNT`
  + `TCM_GETITEMRECT`, paints background / hover / selected per item using
  the cached tab theme, draws the filename, and renders the close `×` —
  giving us a real dark tab strip with a visible close glyph that brightens
  on hover.
- Removed the now-dead `handle_top_tab_custom_draw` and the matching
  `NM_CUSTOMDRAW` arm in `WM_NOTIFY`, plus the unused `NMCUSTOMDRAW`,
  `CDRF_SKIPDEFAULT`, `GetDC`, `ReleaseDC` imports.

## [0.4.11] - 2026-05-15

- Fixed top tab strip ignoring dark mode and showing an invisible close `×`
  (introduced by the v0.4.10 switch to `NM_CUSTOMDRAW`). Comctl32's themed
  paint runs ahead of `CDRF_SKIPDEFAULT` and overwrites our custom paint, so
  `SetWindowTheme(top_tabs, "", "")` is now called right after the tab control
  is subclassed. With visual styles off, the existing `handle_top_tab_custom_draw`
  takes full control of background/hover/selected colors plus the close glyph.
- Lowered the minimum top-tab width from 120 to 80 DPI-scaled px so short
  labels like `new 001` no longer get padded out to ~120 px. Long filenames
  still grow naturally.
- `update_tab_host_theme` now also invalidates `top_tabs` so toggling
  **View &rarr; Dark Mode** at runtime repaints the top strip immediately.
- New `dark_mode::disable_visual_styles(hwnd)` helper wrapping the
  `SetWindowTheme(hwnd, "", "")` ffi.

## [0.4.10] - 2026-05-15

- Replaced the uniform-width top tab bar with per-tab auto-sized tabs (the
  v0.4.9 fix re-applied the size after layout but tabs still rendered
  truncated to `n…` on some sessions). The tab control no longer uses
  `TCS_OWNERDRAWFIXED` — Windows now sizes each tab to fit its filename, and
  `TCM_SETPADDING` reserves space for the close `×` glyph. `TCM_SETMINTABWIDTH`
  keeps narrow filenames readable.
- Re-implemented custom paint via `NM_CUSTOMDRAW` (handled in the parent
  `WM_NOTIFY`) instead of `WM_DRAWITEM`, since `WM_DRAWITEM` requires the now-
  removed `TCS_OWNERDRAWFIXED` style. Theme colors, hover/selection states, and
  the close glyph are unchanged.
- Removed `refresh_top_tab_item_size`, the `TCM_SETITEMSIZE` plumbing, and the
  `TAB_ITEM_MAX_WIDTH` clamp — auto-sized tabs do not need them.

## [0.4.9] - 2026-05-15

- Fixed top tabs truncating filenames to "n…" at startup. Root cause:
  `TCM_SETITEMSIZE` was called from `add_tab` during `WM_CREATE` while the
  tab control still had 0×0 screen dimensions; Windows silently ignored the
  size until the control was shown. Fixed by calling `refresh_top_tab_item_size`
  (and `InvalidateRect`) from `layout_children` after `SetWindowPos` gives the
  tab strip its real dimensions. This fires on both the initial layout and on
  every subsequent `WM_SIZE`, so the correct item size is always applied once
  the control is on screen.

## [0.4.8] - 2026-05-15

- Bumped the per-tab max width clamp from 260 to 400 DPI-scaled px so most
  filenames fit fully in the tab label before any ellipsis truncation kicks
  in. Tabs still grow to fit their text individually; the close `×` stays
  anchored to the right edge.
- Extended dark mode beyond the editor to cover the rest of the window
  chrome, matching Notepad++:
  - **Title bar** now follows the dark theme via
    `DwmSetWindowAttribute(DWMWA_USE_IMMERSIVE_DARK_MODE)` (attribute 20
    on Win10 20H1+ with an attribute-19 fallback for older builds).
  - **Menu bar** background and items are painted dark via the
    undocumented `WM_UAHDRAWMENU` / `WM_UAHDRAWMENUITEM` messages, with
    `SetPreferredAppMode` (uxtheme.dll ordinal 135) +
    `AllowDarkModeForWindow` (ordinal 133) covering popup/context menus.
  - **Status bar** is now subclassed: `WM_PAINT` paints each part with the
    same `tab_host.theme` palette used by the tab strip, with `theme.border`
    separators between parts.
  - **Find / Replace / Find-in-Files / Go-To-Line** dialogs apply dark
    title bars and dark backgrounds on creation; `WM_CTLCOLOR*` returns
    cached dark brushes so Static / Edit / Button / ListBox controls all
    pick up theme colors.
- Added `src/platform/dark_mode.rs` to encapsulate every undocumented
  uxtheme/UAH binding behind safe wrappers that no-op cleanly on older
  Windows builds.

## [0.4.7] - 2026-05-15

- Removed the "Toggle Checkbox" and "Place Checkbox" commands from the editor
  right-click context menu and from the editor command set entirely. The
  `textops/checkbox` module, its helper wrappers in `win32.rs`, and the
  associated `CMD_TOGGLE_CHECKBOX` / `CMD_INSERT_CHECKBOX` constants are all
  gone. Files containing `- [ ] task` style lines continue to display normally
  — the feature was edit-time only with no display side-effects.

## [0.4.6] - 2026-05-15

- Replaced the Hide-Lines / Unhide-All commands with a **Collapse Selection**
  feature available from the editor's right-click context menu. Highlight any
  block of lines, choose `Collapse Selection`, and the first selected line
  stays visible with a clickable `+` marker in the fold gutter; everything
  below it is hidden. Click the `+` to expand. `Expand All Collapsed` in the
  same context menu restores every user-collapsed block in one shot.
- Removed the old `View -> Hide Lines` / `View -> Unhide All Lines` menu items
  and their `Alt+H` / `Alt+Shift+H` accelerators. The underlying
  `SCI_HIDELINES` plumbing is reused by the new collapse path.
- Fixed top tabs truncating to `n...` — every tab now sizes to fit its
  filename (clamped 120-260 DPI-scaled px). Width recomputes on tab open,
  close, rename, and dirty-flag toggle.
- Removed the lighter "buffer bar" beneath the tab strip by subclassing the
  top tab control to paint `WM_ERASEBKGND` and the bottom 3-px seam with the
  active tab theme background. The tab row now meets the editor cleanly in
  both light and dark mode.
- Internal: new Scintilla wrappers for `SCI_MARKERADD` / `DELETE` /
  `DELETEALL` / `GET` and `SCI_GETLINEVISIBLE`; new
  `USER_COLLAPSE_MARKER` (#5) with `SC_MARK_PLUS` shape; fold-margin mask
  extended to render it alongside the lexer fold markers.

## [0.4.5] - 2026-05-15

- Reworked strikethrough back to an indicator-based toggle. Selecting text and
  invoking `Strikeout` (menu, context menu, or new `Ctrl+Shift+X` accelerator)
  now draws a clean strike line with **no `~~` characters added to the buffer**.
  Re-invoking on already-struck text removes the strike. Strike state is
  persisted across close+reopen via the existing `session.json` (no per-file
  sidecar). Duplicate Tab carries strike state into the new tab.
- Removed the v0.4.4 markdown rescan plumbing (`TIMER_MD_STRIKE`, the debounced
  rehighlight loop, `md_strike_pending`/`md_strike_timer` state, the "Too many
  strike matches" status flag) and the `textops/markdown_strike` module.
  Indicator runs are written/read via the restored Scintilla
  `SCI_INDICATOR*` query APIs.
- Added a line-number margin on every editor. Gutter width auto-sizes from the
  current line count via `SCI_TEXTWIDTH(STYLE_LINENUMBER, …)` and re-fits on
  edit so the digits never clip.
- Added a click-sensitive fold margin with plus/minus box markers (Scintilla
  `SC_MASK_FOLDERS` on margin 1). Enabled Lexilla fold properties (`fold`,
  `fold.compact=0`, plus html/preprocessor/comment variants) so all 9 wired
  lexers (cpp/js/json/yaml/powershell/python/html/xml/css/props) get folds for
  free.
- Added Markdown heading-based folds. `.md` / `.markdown` routes to a new
  `LexerKind::Markdown`; fold levels are recomputed on `SCN_MODIFIED` from `#`
  through `######` depth (large-file gated). Lexer styling stays minimal for
  now.
- Contracted folds show inline ` ⋯ N lines ` in a boxed display style. Manual
  `SCN_MARGINCLICK` handling calls `SCI_TOGGLEFOLDSHOWTEXT` so the count is
  computed at toggle time. Existing `Alt+H` Hide-Lines / `Alt+Shift+H`
  Unhide-All commands keep working alongside the new fold UI.
- Themed gutter, line numbers, and fold markers from the existing tab/editor
  theme so light/dark mode switches recolor everything consistently.
- Added `/.claude` and `/agents` to `.gitignore` to stop local working
  directories from cluttering `git status`.

## [0.4.4] - 2026-05-15

- Fixed `Ctrl+S` not saving: the accelerator was bound only to `Ctrl+Shift+S`
  (Save All), so plain `Ctrl+S` fell through to Scintilla and inserted a
  control-character glyph into the editor. Added a dedicated
  `Ctrl+S -> IDM_FILE_SAVE` accelerator.
- Added a persistent close 'x' button on every tab in all three placements
  (Top, Left, Right). Clicking the 'x' routes through the existing `close_tab`
  flow, preserving the dirty-document save prompt.
- Made active-tab highlighting consistent across placements. Top tabs are now
  owner-drawn (`TCS_OWNERDRAWFIXED`) and the vertical `ListView` custom-draw
  handler explicitly fills the full row, so the selected/hover state reads
  cleanly in both modes and tracks `tab_host.theme` (light/dark).
- Replaced the manual toggle-strikethrough command with a markdown-driven
  `Strikeout` command: selected text is wrapped in `~~...~~` and the editor
  re-scans documents on edit to apply the strike indicator to all matching
  spans, with a `Too many strike matches` status flag past the cap. Removed
  the now-unused Scintilla indicator value APIs and session-stored strike
  ranges.
- Added a debounced 250 ms strikethrough re-highlight timer
  (`TIMER_MD_STRIKE`) keyed off `SCN_MODIFIED` to keep editor responsiveness
  during rapid typing.
- Added a `CLAUDE.md` guide at the repo root so future Claude Code sessions
  can orient quickly to build commands and module layout.

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
