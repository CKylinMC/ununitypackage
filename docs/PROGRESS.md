# Progress

Updated: 2026-09-30 (Asia/Shanghai). Branch: next. Version: uup-cli 0.1.0.
Original baseline: 0ea8d40.

## Status

M0–M6 implementation milestones are complete. Rust is the primary project;
the original .NET project is retained under legacy/dotnet. No implementation
milestone is pending. Actual Unity Editor import remains a separate unrun gate.

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

## Verified

Tested implementation commit: 79399b2f57f8bc65b58e2ef2691f34d6ec4d073f.

[Successful native CI run](https://github.com/CKylinMC/ununitypackage/actions/runs/36724536480)

| Job | Result |
| --- | --- |
| Linux native | PASS: format, clippy, regressions, release build, Python interoperability |
| macOS native | PASS: format, clippy, regressions, release build, Python interoperability |
| Windows native | PASS: format, clippy, regressions, release build, Python interoperability |
| Rust 1.88 minimum | PASS: regressions |

Verified downloadable artifacts: uup-cli-Linux-X64, uup-cli-macOS-ARM64 and
uup-cli-Windows-X64. Each contains the executable, README and LICENSE.

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

Implementation checkpoints f356462, 6c8cf2a, d7812e5, bf52144 and 79399b2 were
committed, pushed and checked against remote SHAs. The final documentation
checkpoint records the successful CI result and changes no implementation code.

For subsequent work, follow CLOUD-RUNBOOK.md: inspect local/remote state and this
file before changing anything. The next independent validation task is the Unity
Editor checklist, followed by adding any supplied failing package as a regression.
