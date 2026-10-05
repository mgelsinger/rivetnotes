# Release Checklist

## Prep

- Update `Cargo.toml` version and `CHANGELOG.md`.
- Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test`.
- Run `cargo deny check` (if configured).
- Verify `RIVET_VERBOSE` logging on a clean machine.

## Build + Package

- Run `powershell -ExecutionPolicy Bypass -File scripts/package.ps1`.
- Verify `dist` contains portable zip and `checksums.txt`.
- Verify the portable staging directory and zip contain `THIRD_PARTY_NOTICES/NOTICE.txt`, `THIRD_PARTY_NOTICES/Scintilla.txt`, and `THIRD_PARTY_NOTICES/Lexilla.txt`.
- Run `powershell -ExecutionPolicy Bypass -File scripts/build-installer.ps1`.
- Verify `dist` contains the installer `rivet-<version>-setup.exe`.
- Installer packaging includes `THIRD_PARTY_NOTICES\*` through [`installer/rivet.iss`](../installer/rivet.iss), and portable packaging copies the same directory in [`scripts/package.ps1`](../scripts/package.ps1).
- Test installer per-user and system-wide (elevation prompt).
- Test `scripts/install.ps1` and `scripts/uninstall.ps1` on Windows 10/11.

## Security

- Review dependency updates and licenses.
- Run dependency audit (`cargo audit`) and review RustSec output.
- Run `pwsh -File scripts/generate-third-party-notices.ps1` after lockfile changes,
  then verify with `pwsh -File scripts/generate-third-party-notices.ps1 -Check`.
  Both packages include the generated `THIRD_PARTY_NOTICES/Rust-Crates.txt` report.
- Capture build provenance: toolchain version, git revision, build date.
- If signing is available, sign binaries and update checksums after signing.

## Release

- Attach portable zip and checksums to the release.
- Verify `NOTICE.txt` and the full `THIRD_PARTY_NOTICES/` directory are present in both portable zip and installer.
- Publish release notes based on `CHANGELOG.md`.
