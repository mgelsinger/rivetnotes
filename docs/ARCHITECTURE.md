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
