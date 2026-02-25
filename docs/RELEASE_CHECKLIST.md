# Release Checklist

## Prep

- Update `Cargo.toml` version and `CHANGELOG.md`.
- Run `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test`.
- Run `cargo deny check` (if configured).
- Verify `RIVET_VERBOSE` logging on a clean machine.

## Build + Package

- Run `powershell -ExecutionPolicy Bypass -File scripts/package.ps1`.
- Verify `dist` contains portable zip and `checksums.txt`.
- Run `powershell -ExecutionPolicy Bypass -File scripts/build-installer.ps1`.
- Verify `dist` contains the installer `rivet-<version>-setup.exe`.
- Test installer per-user and system-wide (elevation prompt).
- Test `scripts/install.ps1` and `scripts/uninstall.ps1` on Windows 10/11.

## Security

- Review dependency updates and licenses.
- Capture build provenance: toolchain version, git revision, build date.
- If signing is available, sign binaries and update checksums after signing.

## Release

- Attach portable zip and checksums to the release.
- Publish release notes based on `CHANGELOG.md`.
