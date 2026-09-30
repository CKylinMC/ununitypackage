# Progress

Updated: 2026-09-30 (Asia/Shanghai). Baseline: 0ea8d40.

## Current goal

M6: harden archive boundaries, expand interoperability checks, document usage and
run native platform CI. M0–M5 initial implementation is in place.

## Completed

- Repository/source review and eight legacy synthetic fixture checks.
- Official exporter and UPM format research (see FORMAT.md).
- Rust 1.98.1/.NET 8 prepared; remote read connectivity verified.
- Implementation/specification, test strategy and cloud runbook written.
- M0 pushed as f356462 (remote SHA verified).
- Rust library/CLI implements every specified command, streaming reads,
  transactional writes, resource/raw modifications and UPM conversion.
- 19 integration regressions pass: unknown content, icon/manifest, folders,
  selection, no-temp reads, CRC/truncation, safe paths, mutation preservation,
  long names/PAX attributes, both UPM layouts and stable generated GUIDs.
- cargo fmt, cargo test --locked and strict clippy pass on Linux.

## Validation and limitations

No actual failing user package was provided. Additional archive-extension limits,
independent Python interoperability and public-fixture checks remain to be run.
Unity import checks: NOT RUN (no Editor in this environment).
Native macOS/Windows checks will run through GitHub Actions.

## Next action

Save this functional checkpoint, then harden extended tar metadata/trailing data,
add cross-platform CI and README, and validate native runner results.
