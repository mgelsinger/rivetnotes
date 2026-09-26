# Developing Rivet

[Back to the README](../README.md) | [Contributing](../CONTRIBUTING.md)

## Build from source

### Requirements

- Windows 11 x64
- Rust stable toolchain
- Visual Studio C++ build tools and a Windows SDK for the native editor libraries
- Inno Setup to build or test the installer

### Commands

```powershell
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo run
```

## Project Quality

- CI enforces formatting, linting, tests, and dependency audits
- Unit tests cover core session, settings, text transform, and command behavior
- Build metadata is embedded into `Help -> About Rivet`
- Native data-safety regressions run in CI using hidden windows and an isolated
  profile. The legacy uninstall test confirms that notes survive uninstall.
- Native editing regressions check font and line-number behavior, date insertion,
  save history, deleted-file handling, layout, and dialog backgrounds. CI also
  exercises the actual accelerator table in optimized builds.
- Find in Files displays up to 10,000 matches, skips physical lines over 1 MiB,
  and avoids directory junctions and symbolic links. Narrow the search if a limit
  is reported. Its text decoding currently targets UTF-8 files.
- The [September safety review](SAFETY-REVIEW-2026-09-25.md) records fixes,
  test evidence, and remaining manual checks for version 0.4.25.
- Desktop and updater integration tests can be run with
  `cargo test -- --ignored --test-threads=1`. They require an installed Windows
  spelling provider and internet access, and use a hidden editor with temporary
  user data. Build `cargo build --release --bin rivet` first for the helper test.
  Installer smoke tests also require Inno Setup; run
  `scripts/test-updater-installer.ps1` with `INNO_SETUP_ISCC` set. Add
  `-Scope Machine` from an already elevated test session to check system-wide
  installation. Both scopes must pass before a release is published; interactive
  Windows approval also needs the [manual QA checks](QA-CHECKLIST.md).

See [the architecture](ARCHITECTURE.md), [release checklist](RELEASE_CHECKLIST.md),
and [manual QA checklist](QA-CHECKLIST.md) for more detail.
