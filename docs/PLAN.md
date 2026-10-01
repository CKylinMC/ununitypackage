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

## v2.1.0 extension milestones

Start from the completed 2.0.0 implementation on next. Keep the project named
uup-cli and the executable uup/uup.exe. The user explicitly authorized stable
v2.1.0 publication after the following goals pass.

| Goal | Deliverable / exit condition |
| --- | --- |
| N0 | Commit and push the new command, statistics, metadata and test contracts before implementation |
| N1 | ls/list alias; component-aware path scopes for list/find/extract/info, including virtual directories; existing selectors retain their behavior |
| N2 | info counts and sizes by Unity file category and case-folded extension; scoped text/JSON reports with resource and archive overhead kept separate |
| N3 | Metadata inventory, default summary and safe byte-preserving dump; discover package.json and non-Assets settings with explicit ambiguity handling |
| N4 | Metadata set/edit/remove for discovered files; JSON Pointer edits preserve untouched fields, resource identity and other archive content |
| N5 | Regression checks, interoperability, updated USAGE/SKILL and migration examples; record actual Unity import verification status |
| N6 | Synchronize 2.1.0 versions, push next and annotated v2.1.0; successful native/MSRV release CI and verified published assets |

Commit and push each completed goal and every 10–15 minutes of sustained work.
Record incomplete goals as checkpoints, not completed milestones. Push a release
commit before its tag and never force-move an existing tag.
