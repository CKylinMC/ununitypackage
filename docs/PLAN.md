# uup-cli implementation plan

`uup-cli` (ununitypackage-cli) replaces the .NET executable with a portable Rust
library and CLI. The implementation branch is `next`, based on `0ea8d40`.
Repository ownership/name and the existing WTFPL license remain unchanged.
The project/Cargo package is uup-cli; its binary is uup (uup.exe on Windows).

| Milestone | Deliverable / exit condition |
| --- | --- |
| M0 | Commit specifications, format research, test strategy and cloud runbook before code |
| M1 | Streaming gzip/tar reader, indexed resources, raw entries and diagnostics |
| M2 | info/list/find/cat/show/verify; full and selected safe extraction |
| M3 | Directory pack and streaming repack; unchanged content preserved |
| M4 | add/replace/remove; manifest/icon/cover; non-Assets and raw entries |
| M5 | UPM conversion into Packages or Assets, GUID preservation, dependency report |
| M6 | Regression suite, native Linux/macOS/Windows CI, build artifacts and migration docs |

Use one crate with a library and binary. Separate archive/index, path safety,
query/extraction, transactional writer, mutations, metadata and UPM conversion.
Use pure Rust compression; do not invoke tar, .NET or Unity at runtime.

Each milestone is committed and pushed. Long-running work gets an explicit
checkpoint approximately every 10–15 minutes. Update PROGRESS.md before each
checkpoint; failed checks do not count as completion. No force pushes or master
merges. Consult CLOUD-RUNBOOK.md when resuming.

Unity import behavior is a separate gate from archive correctness. Unity 2022.3
LTS and Unity 6 imports must be reported as unverified when no Editor is available.
See TESTING.md for the independent import checklist.
