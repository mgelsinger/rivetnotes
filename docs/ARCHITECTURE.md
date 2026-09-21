# Architecture

This document defines the intended module boundaries and threading model.
It should be updated as the project grows.

## Module boundaries

- `platform::win32`
  - Owns all Win32 APIs, unsafe code, and FFI definitions.
  - Provides safe wrappers for window creation, message loop, and dialogs.
- `editor::scintilla`
  - Owns Scintilla/Lexilla integration and any related FFI.
  - Exposes a safe Rust API to create and control the editor widget.
  - Scintilla is linked as a static library built from vendored source.
- `app`
  - Application state, command routing, and document management.
- `ui`
  - UI composition (menus, status bar, tabs) using safe APIs only.

## Threading model

- UI and document interactions live on the main thread (Win32 message loop).
- Background work (e.g., find-in-files) uses worker threads with explicit
  cancellation and posts results back to the UI thread.
- No shared mutable state without synchronization; prefer message passing.

## Updates

- `update_protocol` is shared by the application and release-signing utility.
  It verifies an Ed25519 signature over exact JSON payload bytes, a canonical
  stable version, x64 platform, bounded size, and SHA-256 installer digest.
- `app::updates` owns scheduling, cancellation, cache state, and the separate
  helper entry point. UI timers consume worker messages without doing network
  or installer work. The option and last network attempt live in `settings.json`.
- `platform::update_io` wraps WinHTTP, per-user/system uninstall registration,
  and parent-process waiting with PID plus creation-time identity.
- The helper is a copy of the running executable. It starts before shutdown,
  waits for the parent to exit, checks installation identity and unchanged
  source bytes, re-verifies the manifest and payload under a read-only file
  lock, and invokes Inno without UI, reboot, forced closure, or auto-relaunch.
  Only explicit restart requests relaunch Rivet. A failed checkpoint never
  launches the helper. A cancelled normal save leaves the app open.
- The helper uses `%LOCALAPPDATA%\Rivet\updates` for requests, results, staged
  files and an installer log. Old owned cache files are pruned after 30 days.
- Both registered scopes can check and download. Per-user installs can apply on
  normal exit. System-wide installs require an explicit restart action before
  any shutdown handoff or elevation; the helper independently checks that gate.
- For shared installs, the helper invokes only the verified, locked installer
  with `ShellExecuteExW(runas)` and `/ALLUSERS`, then waits on its process handle.
  COM uses an STA and Shell error dialogs are disabled, but Windows UAC remains.
  The helper stays under the original account, so results and a normal-privilege
  relaunch use that account even when another administrator supplies credentials.
- A shared installer uses a unique Inno temporary log instead of an elevated
  write to a predictable user-cache log path. The installer also uses a global
  setup mutex to prevent overlapping setup processes across Windows sessions.
- The release workflow signs final installer metadata with the repository
  secret, uploads all files to a draft, and then publishes the release.

## Spelling

- `editor::spellcheck` owns bounded scan checkpoints, Markdown/address exclusions,
  and UTF-16-to-UTF-8 error-range conversion. It has no Windows dependencies.
- `platform::spellcheck` owns a dedicated COM worker and the installed Windows
  spelling provider. Only owned snapshots and results cross thread boundaries.
- `platform::win32` schedules chunks for the active tab after a typing pause,
  rejects stale results by tab identity/revision/generation, and applies Scintilla
  indicator 10 on the UI thread. Indicators 8 and 9 remain dedicated to smart
  highlighting and strikethrough. Edits invalidate scan checkpoints in the
  affected suffix, including Markdown fence state.
- At most one job is outstanding. Input chunks are at most 16 KiB, and documents
  above 2 MiB or in Large File Mode are excluded. Closing the worker's request
  channel lets it exit after its current bounded job without blocking the UI.
