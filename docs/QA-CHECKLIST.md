# Manual QA Checklist

A short checklist of UI behaviors that don't have automated test coverage
(everything inside `src/platform/win32.rs` is Win32 GUI code). Run through
this before tagging any `v0.x.y` release — the CI workflow only verifies
`fmt`, `clippy`, `test`, and that the portable zip + installer build.

Companion to `docs/RELEASE_CHECKLIST.md`, which covers packaging and
signing.

## Launching

- [ ] `cargo run` launches without an error dialog.
- [ ] The window title bar matches the system theme (dark in dark mode).
- [ ] Menu bar paints with the same theme (light + dark).
- [ ] Status bar paints with the theme; line/col, EOL, encoding, dirty flag
  all show correct values.

## Top tabs (`View → Tabs → Top`)

- [ ] Open 3 files: each tab sizes to fit its filename (no `n…` truncation
  on short names).
- [ ] Active tab uses `selection_bg`; inactive tabs use `bg`; hovered tab
  uses `hover_bg`.
- [ ] Close `×` is visible on every tab and brightens on hover.
- [ ] Clicking `×` closes the tab (dirty-prompt still fires for unsaved
  changes).
- [ ] Opening a file with a very long path: that tab grows to fit; other
  tabs stay at their own widths.
- [ ] Rename or modify a tab so the `*` dirty marker appears/disappears —
  the tab width updates.

## Vertical tabs (`View → Tabs → Left` and `Right`)

- [ ] All open tabs are **visible** (regression guard for v0.4.13–v0.4.14).
- [ ] Only the active tab shows `selection_bg`; others show `bg`.
- [ ] Hovering shows `hover_bg`.
- [ ] Clicking another tab moves the highlight with **no light-mode flash**
  (regression guard for v0.4.16).
- [ ] Close `×` visible on each row and brightens on hover; clicking closes
  the tab.
- [ ] Splitter drag resizes the strip; the new width persists across
  restarts.
- [ ] Middle-clicking a vertical tab closes it (matches Top tabs).

## Dark-mode toggle (`View → Dark Mode`)

- [ ] Toggling at runtime repaints the title bar, menu, status bar, editor,
  top tabs, and vertical tabs without needing a restart.
- [ ] Find / Replace / Find-in-Files / Go-To-Line dialogs honor the current
  theme when opened.

## Editing essentials

- [ ] `Ctrl+S` saves (regression guard for v0.4.4 — a stray glyph used to
  be inserted instead).
- [ ] `Ctrl+Tab` and `Ctrl+Shift+Tab` cycle through tabs with wrap-around.
- [ ] Right-click context menu on the editor shows the expected commands
  with proper enable/disable states.
- [ ] Right-click on a tab shows the tab context menu and actions apply to
  the clicked tab, not necessarily the active tab.

## Session restore

- [ ] Close the app with multiple tabs open (some dirty, some clean) — on
  next launch they restore (named + untitled).
- [ ] Force-kill the app process while editing — backup restore prompt
  fires on next launch and content matches.

## Large File Mode

- [ ] Open a file larger than the threshold (default 20 MB) — status bar
  surfaces the Large File Mode flag, word wrap is off, syntax highlighting
  is suppressed.

## Markdown

- [ ] Open a `.md` file: headings render bold, `**strong**`/`*emphasis*`
  show weight/italic, fenced code and `` `code` `` are tinted, links are
  underlined (light + dark mode).
- [ ] `#` headings get fold markers in the gutter; clicking collapses/
  expands the section.
- [ ] A `# comment` line inside a ``` fenced block or a 4-space-indented
  block does **not** create a fold marker.
- [ ] Typing rapidly in a large `.md` file stays responsive (fold recompute
  is debounced, not per-keystroke).

## Encodings

- [ ] Open a legacy Windows-1252 file (e.g. one containing `é`, `—`, smart
  quotes): it opens (no error dialog) and the status bar shows `ANSI`.
- [ ] `Ctrl+S` on that file keeps it `ANSI`; round-trips without mojibake.
- [ ] Type an emoji into an ANSI file and `Ctrl+S` — a clear error points to
  `Save As` (which writes UTF-8).
- [ ] Open a UTF-16 LE file — status bar shows `UTF-16 LE` (not `UTF-8`).

## Recent Files (MRU)

- [ ] `File → Recent Files` lists files opened via Open dialog, drag-drop,
  and Find-in-Files hits, newest first, capped at 10.
- [ ] Re-opening a listed file moves it to the top without duplicating.
- [ ] Clicking an entry whose file was deleted prunes it and shows a notice.
- [ ] `Clear Recent Files` empties the list; the list survives a restart.

## Spellcheck

- [ ] `View > Spellcheck` is enabled for a fresh configuration and remembers
  being turned off; toggling off clears underlines in every tab immediately.
- [ ] A misspelled word is underlined after typing pauses; correction and
  undo/redo update the underline without affecting other text decorations.
- [ ] Accented and supplementary Unicode characters before a misspelling do
  not shift its underline. CRLF, UTF-16, and ANSI files behave consistently.
- [ ] Markdown inline/fenced code, indented code, link destinations, URLs, and
  email addresses are excluded; prose and link labels are checked.
- [ ] Switching or closing tabs while checking never paints another document's
  results. Reload and Save As refresh spelling eligibility.
- [ ] Large File Mode, documents above 2 MiB, and source formats skip spelling.
  Long wrapped plain-text paragraphs still check words across chunk boundaries.
- [ ] Light/dark mode, zoom, and session restore preserve correct behavior.
- [ ] Missing dictionaries do not block editing; explicitly enabling spellcheck
  explains how to install a supported dictionary.

## Quiet Updates

- [ ] Fresh and upgraded settings keep automatic updates off until enabled.
- [ ] Help-menu checks and download progress stay in the existing status bar.
- [ ] A verified update shows the ready message and enables Restart to update.
- [ ] Normal close installs silently and does not relaunch. Explicit restart
  reopens Rivet with the user's existing session preferences.
- [ ] Cancelled saves, failed backups, and failed session checkpoints postpone
  installation and preserve the edited buffers.
- [ ] Turning automatic updates off cancels downloads and pending installation.
- [ ] Offline checks fail quietly in automatic mode and report inline in manual
  mode. Interrupted or altered downloads cannot become executable updates.
- [ ] Cached updates are verified again after restart. Equal/older versions and
  prereleases are ignored; invalid signatures/hashes are rejected.
- [ ] Portable and script-installed copies do not auto-install.
- [ ] Shared installs offer automatic downloads and explicitly label the restart
  action as requiring administrator approval. Normal close never prompts.
- [ ] Accepting UAC updates the existing shared directory and HKLM registration;
  it does not create a per-user installation. Rivet reopens under the original
  account, including when another administrator supplies credentials.
- [ ] Cancelling UAC reopens Rivet, reports postponement inline, and retains the
  verified update. Other users' running editors are never forcibly closed.
- [ ] An installer failure is reported inline on next launch, without a new
  updater window or a forced Windows restart.
- [ ] `scripts/test-updater-installer.ps1` passes for the release compiler.
- [ ] In an already elevated test session, the same script with `-Scope Machine`
  passes and keeps the installation in HKLM through both versions.

## Tip

If you find a new bug class that this checklist would miss, add a line.
It's easier to extend a living checklist than to re-derive the list from
six months of release notes.
