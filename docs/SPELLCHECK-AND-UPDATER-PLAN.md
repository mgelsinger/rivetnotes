# Spellcheck and auto-updater plan

Assessed September 20, 2026 against Rivet v0.4.22. This is an implementation plan based on repository inspection and current upstream documentation. Neither feature has been implemented or prototyped as part of this assessment.

Follow-up status: spelling underlines, the existing lint fixes, and the optional
updater are implemented under the Unreleased changelog. The implemented updater
uses signed metadata and Rivet's existing status bar; its current behavior and
validation status are documented in [the quiet-updater plan](QUIET-UPDATER-PLAN.md).
The original assessment and estimates below are retained as historical context.

## Repository and release status

- Fetched GitHub branches and tags successfully.
- Local `main`, `origin/main`, GitHub's default branch, and release tag `v0.4.22` all resolve to `e50071fd6849d276e45d4ae62e23bec3fdcf68ff`.
- Ahead/behind count was `0 / 0`; the working tree was clean before this plan was added.
- The [latest Rivet release](https://github.com/mgelsinger/rivetnotes/releases/tag/v0.4.22) has the expected installer, portable archive, and checksums.
- The [September 20 CI run](https://github.com/mgelsinger/rivetnotes/actions/runs/35500205081) passed formatting, tests, and the dependency audit. Clippy failed on `chunks_exact` with constant chunk sizes in `src/app/document.rs:292` and `src/platform/win32.rs:2157`. Resolve those existing lint failures before the next release.

## Recommendation and effort

| Feature | Feasibility | Rough engineering estimate, including focused QA |
| --- | --- | --- |
| Spelling underlines for notes and Markdown prose | Good fit; small to moderate addition | 2-3 working days; basic proof of concept within a day |
| Automatic checks and prompted installation for Inno Setup installs | Good fit using WinSparkle | 3-5 working days, plus any signing or release setup delays |
| Portable updates that replace files in place | Possible; needs additional installation logic | A further 2-4 working days after the installer updater |

These are planning estimates for someone familiar with the code, not measured implementation times. Ship spellcheck first, then the installer updater. Start portable distribution with a manual update route and add replacement in place separately.

## 1. Spellcheck underlines

### Proposed behavior

Show subtle red squiggles under misspelled words. Do not change text automatically or add correction menus in the first release. Include a persisted `View > Spellcheck` toggle. Enable checking for untitled notes, plain text, and Markdown prose by default; leave source-code formats disabled by default.

Use one spelling language at a time, chosen from installed Windows spelling providers. Prefer the user's supported language; use English only if it is available as a fallback. If no suitable provider exists, disable checking gracefully and explain availability when the user explicitly enables it. Do not block opening or editing a document.

### Why this fits Rivet

- `src/editor/scintilla.rs` already wraps indicator styling, filling, and clearing for smart highlighting and strikethrough. Squiggles are another indicator style supported by the vendored Scintilla engine.
- `src/platform/win32.rs` already receives text-change notifications, has delayed timers for Markdown folding and word counts, and uses background work for Find in Files.
- Each `DocTab` has a runtime identity and `change_counter`, which can help reject outdated results.
- The existing `windows` 0.54 dependency already contains `ISpellCheckerFactory`, `ISpellChecker`, and `ISpellingError` bindings under `Win32_Globalization`. Enable that feature instead of introducing a separate spelling library or bundled dictionaries. Microsoft's [factory interface](https://learn.microsoft.com/en-us/windows/win32/api/spellcheck/nn-spellcheck-ispellcheckerfactory) exposes supported languages, and [Check](https://learn.microsoft.com/en-us/windows/win32/api/spellcheck/nf-spellcheck-ispellchecker-check) returns spelling errors.

### Implementation steps

1. **Prove the engine and underline path.** Enable `Win32_Globalization` in `Cargo.toml`. Initialize COM and create a reusable spelling provider on a dedicated worker. Check a short sample and map the returned ranges to Scintilla. Reserve indicator 10 for spelling; existing indicators 8 and 9 must remain independent.
2. **Separate spelling logic from UI wiring.** Add pure range conversion, filtering, and scheduling logic under `src/editor/spellcheck.rs`. Keep COM calls behind a platform wrapper following the repository's FFI boundaries. Pass owned text snapshots to the worker; access Scintilla only on the UI thread.
3. **Handle text positions correctly.** Convert Windows UTF-16 error offsets to Scintilla UTF-8 byte ranges relative to the checked chunk. Validate range bounds and character boundaries. Include accented text, combining marks, supplementary characters, CRLF, and embedded null handling. Underlining must never change document content, the dirty state, or undo history.
4. **Check incrementally.** After roughly 300-500 ms of typing inactivity, check changed paragraphs in bounded chunks. Add a text-range reader so an edit does not copy the entire document. Prioritize the active view, queue initial scans gradually, coalesce pending edits, and suppress incomplete words around the caret while typing. Disable checks in Large File Mode and bound work for long lines and large untitled notes too.
5. **Reject obsolete results.** Tag jobs with tab runtime identity, document revision, checked range, language, and settings generation. Drop results after another edit, reload, tab closure, or language change. Clear obsolete underlines in affected ranges and reschedule invalidated paragraphs. Recheck on activation, load, restore, and Save As when language eligibility changes.
6. **Limit false positives.** Skip URLs, email addresses, Markdown link destinations, inline code, and fenced/indented code blocks. Reuse or extend the existing Markdown fence logic where appropriate, but do not treat its heading-fold output as a complete Markdown parser. Editing a fence must invalidate affected following content.
7. **Persist preferences and finish UI integration.** Extend both `UiSettings` and its custom `UiSettingsWire` deserialization/default conversion. Configure squiggles for light and dark themes and clear all spelling indicators immediately when disabled. Keep suggestions, custom dictionaries, and mixed-language detection for later work.

### Verification and acceptance

- Unit tests for UTF-16/UTF-8 mappings, paragraph invalidation, stale-result rejection, and prose exclusions.
- Provider smoke check with an installed language and graceful handling of a missing provider.
- Manual typing, paste, undo/redo, reload, multi-tab, save/restore, zoom, dark/light theme, and Large File Mode checks.
- Confirm that correcting a word removes its underline, only the intended word is marked after non-ASCII text, and smart highlighting/strikethrough still work.
- Measure typing responsiveness with long notes and rapid edits. Spelling work must stay off the UI thread and queued work must remain bounded.
- Run formatting, Clippy, and existing tests after implementation.

The drawing portion is easy. The main effort is accurate positions, incremental work, and preventing distracting false positives.

## 2. Auto-updater

### Proposed behavior and implementation choice

Use WinSparkle for automatic background checks, update prompts, downloads, and verified installer launch. Include `Help > Check for Updates` and a saved automatic-check preference. Users choose when to install; silent installation during editing is outside the first release.

WinSparkle exposes a C API and ships as one DLL, so it can be wrapped from Rust without replacing the Win32 UI. Its update feed can refer to the existing Inno Setup executable. These are documented in the [integration guide](https://winsparkle.org/guides/integrating-winsparkle/) and [publishing guide](https://winsparkle.org/guides/publishing-updates/). As assessed, the latest upstream release is [v0.9.4](https://github.com/vslavik/winsparkle/releases/tag/v0.9.4); pin and verify the selected x64 package during implementation.

A custom updater would need its own download, verification, installer handoff, and failure UI. WinSparkle avoids much of that work while retaining the current installer. It adds a DLL and feed/signing maintenance to Rivet's distribution.

### Implementation steps

1. **Validate an actual upgrade before full integration.** Use a local test feed and two test versions. Prove the Rust binding, Inno Setup upgrade, user/system installation modes, and shutdown flow. Test with the chosen pinned WinSparkle version.
2. **Add the updater boundary.** Introduce a small safe updater API with FFI isolated in the platform layer. Load the DLL from the application directory using an explicit path; missing updater files should leave editing usable. Initialize after the main window is visible, set version metadata from Cargo/build metadata, and clean up at shutdown. Keep developer builds from checking the production feed by default.
3. **Respect install type.** Identify an Inno installation by its registration and matching executable directory, preferably reinforced by an installer-owned marker. A portable copy must not be mistaken for an installed copy because another Rivet installation exists. The legacy `scripts/install.ps1` copies just the executable into the data directory; treat that path separately and provide a migration/manual update route. Do not launch an Inno upgrade for an unrecognized portable or script-installed copy.
4. **Integrate application shutdown before installer launch.** WinSparkle's [shutdown callbacks](https://winsparkle.org/c-api/callbacks/) execute outside the app's main thread, and its shutdown-request callback happens after installer launch. Marshal the readiness check to the UI thread. Complete saving or recovery snapshots and the session checkpoint before reporting readiness. Reuse `can_exit` behavior while explicitly checking all backup/session preference combinations. A cancellation or write failure must prevent installer launch. Briefly prevent further edits once readiness is accepted, handle launch failure by restoring editing, and avoid callback/cleanup deadlocks.
5. **Preserve installation and data.** Keep the existing Inno AppId, destination, per-user/system scope, shortcuts, file associations, and preferences. Test reduced-UI installer arguments and elevation behavior rather than assuming defaults preserve scope. Add an update-specific relaunch path because the current installer uses `skipifsilent`. Relaunch as the original user after successful installation; do not require an unexpected reboot or forcibly terminate Rivet.
6. **Publish authenticated updates.** Generate an EdDSA key pair, embed only the public key, and keep the private key in a protected release secret with a secure backup. Sign the final installer bytes and publish `sparkle:edSignature` in the feed. WinSparkle documents [download signature verification](https://winsparkle.org/guides/getting-started/); ordinary SHA-256 checksums are useful for integrity but do not replace update signatures. Windows Authenticode signing is a separate distribution improvement; if enabled, perform it before the EdDSA signature and checksums.
7. **Extend release automation.** Update `.github/workflows/release.yml` and packaging to include the pinned DLL and its notices, and copy runtime files in the legacy install script if that script remains supported. Ensure the requested tag is checked out and matches `Cargo.toml`; the current manual workflow selects the release label but does not explicitly check out that tag. Build from that checkout and stop immediately on build/signing failures. Generate checksums for both ZIP and installer after final signing; the current script only hashes the ZIP.
8. **Publish a stable feed last.** Use a fixed HTTPS XML feed in a dedicated `updates` branch of this public repository, with installer URLs pointing to immutable versioned GitHub Release assets. Validate feed redirects, caching, version ordering, architecture, and minimum Windows version in the prototype. Upload and verify the release assets before advancing the feed; keep prereleases off the stable feed. Serialize stable publication and prevent an older release job from replacing a newer feed. Document how to withdraw a bad release and publish a corrective higher version.
9. **Finish user controls and rollout.** Use one source of truth for the automatic-check preference across settings and WinSparkle. Manual checks should report current version or failure; background failures should be quiet and logged. Provide skip/remind behavior. Existing v0.4.22 users need one manual installation of the first updater-enabled version before future automatic updates can work.

### Portable support

For the first updater release, expose a clearly labeled manual download route for portable and legacy script installs. Do not describe this as automatic replacement of portable files.

A subsequent portable updater needs a signed portable payload, staging directory, safe archive extraction, writable-directory checks, and a small helper that waits for Rivet to exit before replacing its files. Back up the previous program files, roll back if replacement fails, and restart with the existing session. Restrict replacement to a manifest of program files and preserve all user files, settings, sessions, and backups. Use a separate portable feed/channel so a ZIP is never treated as an Inno installer. Test locked files, removable media, interruption, and rollback before enabling this path.

### Verification and acceptance

- Upgrade between two real packaged versions in both per-user and system-wide modes, including a custom install directory.
- Preserve dirty named files, untitled notes, tabs, settings, and restored sessions across an update. Verify Cancel and failed-save paths prevent installation, including when session restoration or periodic backups are disabled.
- Reject a tampered download, missing/invalid signature, wrong architecture, and inappropriate older version. Test offline operation, interrupted downloads, malformed feeds, and installer launch failure.
- Confirm portable detection, missing-DLL handling, correct elevation/relaunch user, and no duplicate installation or unwanted shortcut changes.
- Run a release rehearsal in a disposable Windows environment before pointing the stable feed at production updates.

## Delivery order

1. Resolve the two existing lint failures and establish passing release checks.
2. Implement and verify spelling underlines with Windows' installed provider.
3. Prototype WinSparkle and the existing installer together, especially the save-and-exit handshake.
4. Add signing, feed publication, packaging, updater settings, and upgrade QA; ship the installer updater.
5. Add automatic portable replacement only after its helper and rollback path are verified.

The only repository change made for this assessment is this plan. No application code, installer behavior, release assets, or GitHub branches were changed.
