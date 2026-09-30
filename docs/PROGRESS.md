# Progress

Updated: 2026-09-30 (Asia/Shanghai). Baseline: 0ea8d40.

## Current goal

M6: run native platform CI and public-fixture compatibility checks. M0–M5 are
implemented; hardening, interoperability and usage documentation are complete.

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
- Functional checkpoint 6c8cf2a pushed and verified.
- 29 regressions now pass, including extension bounds, trailing-data rejection,
  PAX size overrides, long links, malformed PNG, raw recovery and case-alias GUIDs.
- Independent Python tarfile USTAR/GNU/local-PAX read/repack/extract checks pass.
- Original tar names (including ./), supported attributes and long link targets
  are preserved. Empty unknown directories are retained.
- README documents every command, compatibility boundaries and migration.
- .NET sources/projects archived under legacy/dotnet; Rust is the primary project.
- Native three-platform CI and Rust 1.88 minimum job configured, awaiting remote runs.

## Validation and limitations

No actual failing user package was provided. Public fixture checks remain to run.
Unity import checks: NOT RUN (no Editor in this environment).
Native macOS/Windows checks will run through GitHub Actions.

## Next action

Push this checkpoint, inspect GitHub Actions jobs and fix any native failures.
Validate public Unity-generated fixtures, update results and push final progress.
