# Project Constraints

Target: Windows 10/11 x64 only.

Non-goals: plugins, docking layout, split views, embedded terminal, LSP, macro recorder.

Priorities (in order): correctness, fast startup, stability, small binary, predictable UX, clean codebase.

License: pick early (MIT/Apache-2.0 recommended) and apply uniformly.

"Simple but perfect" definition: minimal feature set with zero papercuts in those features.

# Phase 0 - Repo + build system + policy

Create repo rivet.

Set up CI (GitHub Actions) for:

- cargo fmt (required)
- cargo clippy -D warnings
- cargo test

Release build artifact for Windows x64.

Add code quality gates:

- rustfmt.toml (default is fine)
- clippy deny list
- deny.toml (cargo-deny) optional

Define contribution rules:

- Conventional commits optional; at least enforce "no merge if CI fails."

Establish security posture:

- No auto-updater in MVP (reduces risk surface).
- Reproducible builds target (later): pinned toolchain via rust-toolchain.toml.

Add docs:

- README with scope, goals, non-goals, build steps.
- docs/ARCHITECTURE.md with module boundaries and threading model.

# Phase 1 - UI stack decision (Windows-native)

Use Win32 windowing (menus, accelerators, common dialogs) via Rust windows crate.

Editor component: embed Scintilla (and Lexilla if needed for lexers), compiled/linked locally.

Decide integration approach:

- Preferred: build Scintilla as a static lib (or ship scintilla.dll) and host as child window.

Establish a single abstraction layer:

- platform::win32 module owns all unsafe/FFI; the rest is safe Rust.

# Phase 2 - Skeleton application (boot to empty editor)

Implement:

- WinMain equivalent entry -> create main window.
- Message loop + WndProc dispatch.
- Child Scintilla control creation and resize handling.
- Status bar (initially basic) + minimal menu (File/Edit/View/Help).

Hard requirements:

- No panics on startup; graceful error dialog + exit.
- Startup time benchmark harness (even simple stopwatch logging).

Deliverable:

- App launches, shows editor, typing works, resize works.

# Phase 3 - File I/O foundation (correctness first)

Implement robust open/save:

- Common file dialogs (Open/Save As).
- Drag & drop open.
- File change detection on disk (basic: timestamp + size; later optional watcher).

Encoding handling (minimal but correct):

- UTF-8 (with/without BOM), UTF-16 LE/BE detection.
- Save options: UTF-8 (no BOM default), UTF-8 BOM optional, UTF-16 LE optional.

Line endings: preserve existing; allow convert (CRLF/LF).

Large file strategy:

- Define an explicit threshold behavior (e.g., >50-200MB):
- Disable heavy features (word wrap, full syntax highlighting) and show "Large File Mode".

Deliverable:

- Open/save round-trips without corruption across encodings/EOLs; handles "file changed on disk" prompt.

# Phase 4 - Tabs + session (the core Notepad++ expectation)

Add tab bar:

- Multiple documents.
- Close tab, close others, close to right.
- Unsaved indicator "*".

Session restore:

- Persist open files + caret position + encoding + EOL + view prefs.
- Restore on next launch; handle missing files gracefully.

Crash-safety baseline:

- Periodic session checkpoint (metadata only, not file content) to avoid losing state.

Deliverable:

- Smooth multi-tab workflow; session reliably restores.

# Phase 5 - Editing essentials (simple but complete)

Implement expected Edit features:

- Undo/redo, cut/copy/paste, select all.
- Duplicate line, delete line, move line up/down.
- Indent/outdent selection (tab/shift-tab).
- Trim trailing whitespace (command).

Keyboard shortcuts:

- Define a small, standard set (Notepad++-adjacent but not necessarily identical).
- Use accelerators at Win32 level + map to commands.

Clipboard correctness:

- Preserve Unicode; handle large clipboard data safely.

Deliverable:

- Editor feels "done" for day-to-day edits without surprises.

# Phase 6 - Find/Replace (including Find in Files)

In-document:

- Find next/prev, replace, replace all.
- Options: case, whole word, regex (optional but valuable), wrap.

Find in Files:

- Folder picker, include/exclude patterns, recursion toggle.
- Results panel with clickable matches (file + line preview).
- Cancellation support for long searches.

Performance:

- Streaming search for large trees; avoid loading entire files when possible.

Deliverable:

- Reliable searching comparable to Notepad++ for typical workflows.

# Phase 7 - Syntax highlighting (minimal set, fast)

Decide highlight scope:

- Ship a small curated set: plaintext, JSON, XML, INI, YAML, PowerShell, Python, JS/TS, HTML/CSS, C/C++.
- Use Lexilla lexers where practical; keep themes simple.
- Large File Mode disables or reduces lexing automatically.

Deliverable:

- Highlighting is fast, consistent, and never causes hangs.

# Phase 8 - UI polish without scope creep

Dark mode:

- Start with editor + background + text + caret + selection.
- Then menus/tabs/status bar if feasible; if not, provide "Editor dark / UI light" option explicitly.

DPI correctness:

- Per-monitor DPI awareness v2; scale icons, padding, hit targets.

Accessibility:

- Keyboard-only operation complete for all commands.
- Reasonable screen reader support where available (don't break standard controls).

Deliverable:

- Looks professional and behaves correctly on modern Windows displays.

# Phase 9 - Reliability, instrumentation, and guardrails

Logging:

- Local log file with rotation; opt-in verbose mode.

Telemetry:

- None in MVP.

Error handling:

- Central error type; user-friendly dialogs; never silent data loss.

Fuzz/edge testing focus areas:

- Encoding detection, newline conversions, find/replace regex, large file mode transitions.

Deliverable:

- "Boringly stable" editor.

# Phase 10 - Packaging and release

Provide:

- Signed binaries (if possible), otherwise clear checksums.
- MSI or MSIX or a simple installer + portable zip (portable is often appreciated).

Versioning:

- Semantic versioning; changelog.

Security release checklist:

- Dependency audit, reproducible build notes, build provenance if possible.

Deliverable:

- First public release candidate.

# Definition of Done for MVP (use as acceptance criteria)

- Opens/saves reliably with correct encoding + EOL preservation.
- Multi-tab with session restore.
- Find/replace + find-in-files works and can be cancelled.
- Syntax highlighting for a curated set, auto-disabled in Large File Mode.
- Stable, fast startup, no crashes in normal use, clear prompts for destructive actions.
- No plugin system, no updater, minimal attack surface.

# Execution format for developer agent

For each phase, require the agent to deliver:

- A PR-sized change set (even if local): summary, file list, screenshots/GIF where relevant.
- A short "how to test" checklist.
- Explicit notes on unsafe/FFI touched (must be isolated in platform::win32 and editor::scintilla).
- Bench notes for anything performance-sensitive (startup, open large file, find-in-files).
