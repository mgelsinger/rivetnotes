# Rivet safety and reliability review

Review started September 25, 2026, against public release 0.4.23
(`ce7cce2f0a2d9741f0def5808deccdcdbcb058a1`). The changes described here are
included in version 0.4.24. See the [release](https://github.com/mgelsinger/rivetnotes/releases/tag/v0.4.24)
for publication status and downloadable assets. Testing used isolated profiles
and did not replace the user's installed copy.

## Assessment

The review found concrete data-loss risks in existing save, recovery, tab-close,
and legacy uninstall behavior. Those issues warrant a patch release before
promoting the current release more widely. The fixes below have local regression
coverage. Spellcheck's bounded worker design and the updater's signature, hash,
scope, and process checks are sound foundations; no spelling suggestions or
additional updater windows were added.

This was a source review and regression-testing pass, not a penetration test,
formal verification, or exhaustive audit of every vendored C++ source file.

## Findings and changes

| Priority | Finding | Resolution |
| --- | --- | --- |
| High | Manual saves truncated the destination before writing finished. | Stage and flush complete contents, then replace the destination. Preserve original access lists and named streams. Keep completed recovery files on replacement failure and report their paths. Existing symbolic-link targets are resolved before saving. |
| High | Loading text through a NUL-terminated API could silently drop everything after an embedded NUL. | Use a length-bearing Scintilla replacement operation. Test editing, backup, restoration, and byte-for-byte saving in all five supported encodings. |
| High | Dirty backups lost the original disk encoding on restoration. | Persist encoding and EOL in backward-compatible optional session fields. Infer encoding from disk for older snapshots when possible. |
| High | Closing a dirty tab with snapshots enabled deleted its recovery data without asking. | Always require a Save/Discard/Cancel choice for dirty-tab closure. Normal app closure still follows the user's snapshot preference. |
| High | Exit discard choices could be lost or interfere with cancellation; normal exits did not require a successful final checkpoint. | Apply discard choices only to the committed exit snapshot. Keep buffers intact if a later prompt cancels. Require successful checkpoints before normal close/update handoff. |
| High | Windows shutdown bypassed the normal close/recovery path. | Handle shutdown query, completion, and cancellation, preserving recovery state without starting an updater during sign-out. |
| High | Malformed sessions could be replaced by an empty session; unreadable entries could disappear from future checkpoints. | Preserve malformed JSON under a unique recovery name. Retain entries that fail restoration so they can be retried on the next launch. Recompute backup paths inside the current profile. |
| High | The legacy uninstall script recursively removed a directory that also contained user notes and preferences. | Remove only the legacy executable and its own shortcut. An isolated regression confirms preservation of notes, settings, and another installation's shortcut. |
| Medium | Find in Files could panic while truncating Unicode, accumulate unlimited results/line buffers, follow directory junction loops, or revive a cancelled worker. Invalid regexes silently became literal searches. | Use UTF-8 boundary checks, a 10,000-result cap, 1 MiB line cap, reparse-point exclusion, per-search cancellation flags, validated regexes, and bounded UI result batches. |
| Medium | A disconnected updater worker could leave the controller permanently busy. | Clear the failed worker and allow later checks; report failure inline for manual checks. |
| Medium | The native editor was missing upstream crash and editing fixes; the lockfile included a dependency advisory. | Refresh Scintilla 5.5.2 to 5.6.6 and Lexilla 5.4.7 to 5.5.3. Preserve the custom lexer catalogue. Update locked anyhow to 1.0.103. Record archive hashes in third-party notices. |
| Low | Backup-directory scans ran on every checkpoint; wildcard matching allocated a full matrix; unused fields and eager spelling startup added overhead. | Scan at startup, use one matching row, remove unused document fields, and create the spelling worker only for eligible text. |
| Low | Log rotation happened only at startup; Git metadata changes rebuilt native code unnecessarily; header changes could be missed. | Rotate during long sessions, limit individual log records, and correct build dependency tracking. |
| Defense in depth | Native open-file messages could request large allocations before their type and size were validated. | Check the message type, byte limit, UTF-16 alignment, and null data before copying. |

Atomic replacement does not guarantee durability against every storage-device or
power failure. Permissions are applied to the empty staging file before content
is written, and EFS encryption attributes are requested for encrypted targets.
Completed `.rivet-recovery.*` files are excluded from routine stale-temp cleanup.
Failed periodic snapshots show an inline warning and wait a minute between
automatic retries; final close retries immediately. Retained recovery files can
consume disk space during persistent storage failures and should be reviewed
after recovering the notes. Cleanup validates the generated temporary-name
format so similarly named ordinary files are not swept up.

## Updater and spellcheck checks

The updater verifies signed metadata, version/platform constraints, installer
size and hash, and cached content again before use. HTTPS downloads are bounded.
The helper checks installation identity and the parent process's creation time,
waits for exit, and holds the installer read-only while executing it. System-wide
installation still requires explicit restart and Windows approval. Portable and
legacy script installations cannot silently install updates.

Spelling uses a dedicated Windows COM worker, bounded chunks, revision/generation
checks before applying results, and UTF-16-to-UTF-8 position mapping. It does not
change the text or undo history. Tests exercise Unicode offsets, prose/code
exclusions, tab switching, toggling, and editor integration.

## Validation

Local Windows checks for this revision:

| Check | Result |
| --- | --- |
| `cargo fmt --check` | Pass |
| `cargo clippy --all-targets --locked -- -D warnings` | Pass |
| `cargo test --locked` | 142 regular tests passed; 10 opt-in integration tests |
| Explicit integration tests, serialized | Pass, including native spelling, editor behavior, recovery, shutdown messages, HTTPS bounds, and unsigned helper-request rejection |
| Release executable build | Pass |
| `cargo audit --deny warnings` | Pass after updating the advised dependency; 110 locked dependencies scanned |
| Legacy uninstall with isolated profile and shortcuts | Pass |
| Isolated per-user installer install/upgrade | Pass: correct payload and registration, no installer window/relaunch, installer file lock, and running-app/setup mutex checks |
| Real system-wide install/UAC in this review | Not repeated on the user's device; both installer scopes remain release workflow gates |

Tests use temporary profiles and hidden windows. They do not load or replace the
user's actual notes or installation. Native data-safety and legacy-uninstall
tests now run in CI and the release workflow. Dependency advisories, including
warnings, now fail CI and block release publication.

## Remaining scope and release checks

- Version 0.4.24 contains these fixes; 0.4.23 does not. The release workflow
  builds the exact version tag and requires its checks to pass before publication.
- Complete visual light/dark, DPI, tab-layout, IME, and accessibility checks on
  the release candidate. Hidden controls verify behavior but not appearance.
- Use a disposable Windows account or VM for real sign-out/restart and interactive
  system-wide UAC acceptance/cancellation. Shutdown was simulated through native
  messages, not by signing out of the user's session.
- Exercise low-disk-space/power interruption, network shares, cloud-synced files,
  and EFS-encrypted files on representative systems. Locked-file failure, access
  list preservation, and named streams are covered; these environments are not.
- Find in Files still targets UTF-8. UTF-16/ANSI search parity is useful future
  work; the editor's open/save support for those encodings is separately tested.
- Large File Mode is selected on load/save. Pasting a very large buffer can still
  make non-spelling features expensive before the next save. Spelling has an
  independent 2 MiB limit. Dynamic large-buffer handling is follow-up work.
- The Win32 UI is a large module with raw state pointers and synchronous,
  reentrant callbacks. Its ownership/lifetime model deserves a dedicated refactor
  and stress testing. Passing tests are not proof that all unsafe boundaries are
  free of lifetime or reentrancy defects.
- Authenticode signing of the public executable/installer is separate from the
  updater's signed metadata and remains a distribution-quality improvement.

See [manual QA](QA-CHECKLIST.md) and [release checklist](RELEASE_CHECKLIST.md).

## Upstream references

- [Scintilla release history](https://www.scintilla.org/ScintillaHistory.html)
- [Lexilla release history](https://www.scintilla.org/LexillaHistory.html)
- [Windows ReplaceFile behavior and failure modes](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew)
- [Windows shutdown query](https://learn.microsoft.com/en-us/windows/win32/shutdown/wm-queryendsession)
- [Windows shutdown completion](https://learn.microsoft.com/en-us/windows/win32/shutdown/wm-endsession)
- [anyhow advisory RUSTSEC-2026-0190](https://rustsec.org/advisories/RUSTSEC-2026-0190)
