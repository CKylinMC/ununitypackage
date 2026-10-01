# Progress

Updated: 2026-10-01 (Asia/Shanghai). Branch: next. Version: uup-cli 2.1.0.
Original baseline: 0ea8d40.

## Status

Active goal: user-authorized v2.1.0 extension and stable release. N0 documentation
checkpoint 321db22 is pushed and its remote SHA verified. N1/N2 shared scopes,
ls alias and info statistics are implemented. Local format, strict clippy and
35 integration regressions PASS. New cases verify Unicode/virtual directories,
component boundaries, selector intersections, raw scopes, metadata inclusion and
disjoint category/extension count and byte sums. N3/N4 metadata inventory,
default summary, safe dump, resource/raw settings and JSON Pointer editing are
implemented; local format/strict clippy and 43 regressions PASS. Summary/get/list
run without a writable temporary directory. Tests verify ambiguity handling,
PNG header summaries, malformed/oversized JSON, dump conflicts/symlinks, pointer
escaping/arrays, failed-edit preservation, identity and unknown byte retention.
N3/N4 checkpoint cb23e5e is pushed and its remote SHA verified. N5 usage and Agent
documentation now covers scopes, statistics schema/categories, discovered kinds,
summary/dump, selectors, pointer rules and opaque settings. Thirty-two distinct
literal new-feature examples, two embedded Python examples, frontmatter and local
links PASS on generated fixtures. Independent Python USTAR/GNU/local-PAX checks
now additionally verify scope/statistics and metadata dump/edit/unchanged bytes;
all three PASS. Seven release-script tests PASS and the release build PASS.
N5 checkpoint bc39519 is pushed and its remote SHA verified. Rust 1.88 also PASS:
43 regressions against the new implementation. Cargo/lock/current version examples
are now synchronized to 2.1.0. Remote next matches bc39519 and v2.1.0 is absent.
Final 2.1.0 format/strict clippy, 43 regressions, release build, all three Python
formats, seven release tests, tag/Cargo alignment and 32 CLI/two Python examples
PASS. The local Linux bundle is created with uup 2.1.0 and current documents.
Release commit 7d0a45014a989d2537524a8c3eeb8befa4e002a7 is pushed and its remote SHA
verified. Annotated v2.1.0 is pushed; remote tag object is
1611a377fd7e11b924739c6e5d25f1b64fc373a2 and its peeled SHA equals that exact release
commit. No tag was moved. At this checkpoint the new Actions run has not yet
appeared in the API (event propagation); do not mistake the historical v2.0.0 run
for v2.1.0 verification. Next: locate the v2.1.0 release run, observe all native/
MSRV/publication jobs, then download every bundle/checksum and verify published
stable status, architectures, binary names, documentation and version output.
Baseline next is 56f4226; v2.1.0 must be created only after checks pass. Existing
2.0.0 validation below is historical and is not evidence for the new features.

| v2.1.0 goal | Status |
| --- | --- |
| N0 specs before code | Complete; 321db22 pushed and verified |
| N1 scopes/ls | Complete locally; native release CI pending |
| N2 info statistics | Complete locally; native release CI pending |
| N3 metadata discovery/dump | Complete locally; native release CI pending |
| N4 metadata edits | Complete locally; native release CI pending |
| N5 tests/docs | Complete locally: 43 regressions, three formats, 32 CLI + 2 Python examples |
| N6 2.1.0 release | Commit/tag pushed and SHA-verified; native/publication pending |

M0–M6 implementation milestones are complete. Rust is the primary project;
the original .NET project is retained under legacy/dotnet. The requested usage
guide and binary-name correction are complete and have passed native CI.
Actual Unity Editor import remains a separate unrun gate.

## Completed: 2.0.0 tag-triggered release

The original remote v1.0.0 release was confirmed. Cargo.toml/Cargo.lock and
versioned examples now use 2.0.0. AGENTS.md and RELEASING.md persist the user's
future release policy: use specified versions first, otherwise select by SemVer
scope or clarify uncertainty, and synchronize versions/tags before publication.

Actions now triggers only on v* tags. It validates version alignment, classifies
SemVer and legacy alpha/beta/rc suffixes, runs native/MSRV checks, bundles root
uup/uup.exe with docs and checksums, then generates notes and publishes a Release.
Regular branch/PR pushes no longer build. Local validation passed: 7 release tests,
stable/prerelease/re-run publisher argument checks, workflow structure, docs links,
32 Rust regressions, format, strict clippy, release build and three independent
tar formats. The actual binary reports uup 2.0.0; the Linux release bundle has
the root executable with 0755 mode, all documents and a SHA-256 companion.

Release commit 8a07187296c64d67f904825d371ed59375e03828 was pushed to next.
Annotated v2.0.0 was pushed; remote tag object is 4facd5a9f40ea8fdd69be9140db920fb85fb733f
and its peeled SHA matches that exact commit. Tag push triggered
[Release workflow](https://github.com/CKylinMC/ununitypackage/actions/runs/36753329704),
completed successfully: version validation, all three native jobs, Rust 1.88 and
the publication job passed. [v2.0.0 Release](https://github.com/CKylinMC/ununitypackage/releases/tag/v2.0.0)
is published with draft=false and prerelease=false. GitHub-generated notes link
the v1.0.0...v2.0.0 changelog. All three platform archives and SHA-256 companions
are present. No tag was moved; later progress commits only update next.

All published bundles were downloaded and verified: SHA-256, root uup/uup.exe,
six bundled files and Linux x86_64/macOS ARM64/Windows x86_64 binary headers.
Unix executable modes are 0755; the downloaded Linux executable reports uup 2.0.0.
The release is complete. Future development creates no tags until a release is
requested, following AGENTS.md and RELEASING.md.

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
At that earlier checkpoint, direct Actions ZIP inspection was unavailable
(HTTP 403). The newer published Release downloads were inspected successfully
as recorded above.

## Verified

Agent skill documentation validation: PASS. SKILL.md frontmatter and local links
were checked; 41 literal CLI examples ran on generated fixtures, every resulting
package passed verify, and the embedded Python example correctly selected by GUID
and kept payload bytes separate from JSON. This follow-up changes documentation
and the artifact file list only; no Rust implementation changed.

Tested release commit: 8a07187296c64d67f904825d371ed59375e03828 (v2.0.0).

[Successful release CI run](https://github.com/CKylinMC/ununitypackage/actions/runs/36753329704)

| Job | Result |
| --- | --- |
| Version validation | PASS: seven release tests and Cargo/tag alignment |
| Linux native | PASS: format, clippy, regressions, release build, Python interoperability |
| macOS native | PASS: format, clippy, regressions, release build, Python interoperability |
| Windows native | PASS: format, clippy, regressions, release build, Python interoperability |
| Rust 1.88 minimum | PASS: regressions |
| GitHub Release | PASS: complete assets/checksums, generated notes and stable publication |

Verified Release archives: uup-v2.0.0-linux-x86_64.tar.gz,
uup-v2.0.0-macos-aarch64.tar.gz and uup-v2.0.0-windows-x86_64.zip, each with a
.sha256 asset. Bundles contain uup/uup.exe at root, README, SKILL.md, LICENSE,
docs/USAGE.md and docs/RELEASING.md.

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

Implementation checkpoints f356462, 6c8cf2a, d7812e5, bf52144, 79399b2, 3cbd5cb and
release 8a07187 were committed, pushed and checked against remote SHAs. Checkpoint
ff1bd51 recorded the published tag and running release job. The final documentation
checkpoint records successful release delivery and changes no implementation code.

For subsequent work, follow CLOUD-RUNBOOK.md: inspect local/remote state and this
file before changing anything. The next independent validation task is the Unity
Editor checklist, followed by adding any supplied failing package as a regression.
