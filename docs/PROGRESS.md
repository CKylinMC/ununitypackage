# Progress

Updated: 2026-10-01 (Asia/Shanghai). Branch: next. Version: uup-cli 0.1.0.
Original baseline: 0ea8d40.

## Status

M0–M6 implementation milestones are complete. Rust is the primary project;
the original .NET project is retained under legacy/dotnet. The requested usage
guide and binary-name correction are complete and have passed native CI.
Actual Unity Editor import remains a separate unrun gate.

## Delivered

- Repository review, eight legacy synthetic checks, format research, plan,
  CLI specification, verification strategy and cloud recovery runbook.
- Rust library and CLI: info, list, find, cat, show, extract, pack/build, repack,
  add, replace, remove, metadata, from-upm and verify.
- Streaming read-only operations, explicit resource/GUID/raw selectors, JSON
  output, bounded archive processing and actionable diagnostics.
- Full/selected extraction with optional metas, empty folders, non-Assets paths,
  icons, manifests and unknown content. Unsafe paths and output conflicts fail.
- Transactional archive output, preservation of unmodified payloads, GUIDs,
  metas, previews, original tar names and supported extension attributes.
- Raw-entry and resource editing, independent manifest/icon/cover operations,
  complete UPM collection and both Packages and Assets mapping modes.
- Stable generated GUIDs when explicitly requested, preserved existing importer
  settings, dependency and compatibility reports without fetching dependencies.
- Portable archive inspection, native output-filename validation, trusted macOS
  system aliases and symlink guards. Unclassified asset.meta entries stay opaque.
- Usage/migration documentation, independent format checks and native CI artifacts.
- Standalone Chinese usage guide (USAGE.md) with all commands, selectors, workflows
  and a source-checked comparison against the legacy extract/build interface.
- Root SKILL.md with Agent task selection, selectors, workflow examples, structured
  output/byte handling, recovery guidance and validation boundaries. Future CI
  artifacts include this skill alongside the executable and usage guide.

## Follow-up: usage guide and executable name

The user specified that only the project is named uup-cli; the executable must be
uup / uup.exe. Cargo binary, clap help, regression executable lookup, CI paths and
all current command examples are updated. The GUID generation namespace retains
its existing project identity. CI artifacts now also include docs/USAGE.md.

Local verification: PASS, 32 regressions, format, strict clippy, release build and
independent USTAR/GNU/local-PAX checks. An offline cargo install installed only
bin/uup, with version/help named uup. Forty one-time usage workflow invocations
passed, including build/cover migration, selectors, mutation and both UPM layouts;
documentation links were checked.

Follow-up commit 3cbd5cb was pushed and its remote SHA verified. Linux, macOS,
Windows and Rust 1.88 jobs all passed in the native run recorded below. GitHub's
API confirms all three artifacts are available. Native interoperability commands
execute uup / uup.exe, and upload paths include those binaries and USAGE.md.
Direct ZIP inspection from this cloud host was unavailable (artifact download
returned HTTP 403); no claim of downloaded-binary inspection is made.

## Verified

Agent skill documentation validation: PASS. SKILL.md frontmatter and local links
were checked; 41 literal CLI examples ran on generated fixtures, every resulting
package passed verify, and the embedded Python example correctly selected by GUID
and kept payload bytes separate from JSON. This follow-up changes documentation
and the artifact file list only; no Rust implementation changed.

Tested implementation commit: 3cbd5cb4f953cab5cce7966facda27245c7b6386.

[Successful native CI run](https://github.com/CKylinMC/ununitypackage/actions/runs/36746557879)

| Job | Result |
| --- | --- |
| Linux native | PASS: format, clippy, regressions, release build, Python interoperability |
| macOS native | PASS: format, clippy, regressions, release build, Python interoperability |
| Windows native | PASS: format, clippy, regressions, release build, Python interoperability |
| Rust 1.88 minimum | PASS: regressions |

Verified downloadable artifacts: uup-cli-Linux-X64, uup-cli-macOS-ARM64 and
uup-cli-Windows-X64. Upload paths are target/release/uup or target/release/uup.exe,
README, docs/USAGE.md and LICENSE.

Local Linux validation: 32 integration regressions pass; cargo fmt --check,
strict clippy, release build and independent Python USTAR/GNU/local-PAX checks
pass. Platform-specific tests run in their corresponding native CI jobs.

Public Cobertos fixtures test.unitypackage, testo.unitypackage and
testLeadingDots.unitypackage pass verify. Public security fixtures are plain tar;
they were wrapped with gzip only for checks. Invalid output filenames remain
inspectable, and unsafe pathnames are diagnosed. Third-party payloads are not
committed; regression fixtures are generated from explicit test data.

## Separate verification and limits

- Unity 2022.3 LTS and Unity 6 Editor import: NOT RUN (Editor unavailable).
  The TESTING.md checklist covers UI icons, dependency/importer behavior,
  resource references, non-Assets imports and the legacy duplication report.
- No failing user package was provided; additional real-world compatibility
  cases can be added when one becomes available.
- Global PAX headers and sparse formats are explicitly rejected. Container bytes
  may change on rewrite; unmodified supported entry content is preserved.
- Generated metas are minimal; existing importer configuration is preserved.
- macOS native CI uses ARM64. Intel macOS can build from source but has not been
  tested on a native runner in this delivery.

## Checkpoints and continuation

Implementation checkpoints f356462, 6c8cf2a, d7812e5, bf52144, 79399b2 and 3cbd5cb
were committed, pushed and checked against remote SHAs. Documentation checkpoint
683b087 recorded the original CI result. The latest documentation checkpoint
records the renamed binary's successful CI and changes no implementation code.

For subsequent work, follow CLOUD-RUNBOOK.md: inspect local/remote state and this
file before changing anything. The next independent validation task is the Unity
Editor checklist, followed by adding any supplied failing package as a regression.
