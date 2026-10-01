# Verification strategy

## v2.1.0 acceptance matrix

- ls/list equivalence; positional/flag scopes; file and virtual-directory scopes;
  component boundaries, Unicode, raw scopes, no-match exit 3 and legacy selectors.
- Scoped extraction keeps metas, excludes siblings and preflights conflicts.
- info checks case-folded extensions, all categories, count/byte sums, unknown and
  extensionless files, folders/overhead separation and non-Assets resources.
- metadata default summary/list/get run without a writable temporary directory;
  discovery covers fixed entries, multiple package.json files, project manifest,
  resource/raw settings, malformed JSON and oversized summaries.
- dump preserves bytes and logical paths, detects existing files, duplicates,
  symlinks and file/directory conflicts before creating any output.
- set/edit/remove preserve GUID/meta/preview/unknown bytes; JSON Pointer tests
  cover escaping, parent creation, arrays and invalid edits; failed mutations
  preserve source and existing destination. YAML/binary settings use full set.
- Update usage and Agent examples, run format/clippy/Rust/release tests and the
  independent Python interoperability check before tagging. Native Linux/macOS/
  Windows and Rust 1.88 run on v2.1.0 before automated Release publication.
- Verify published stable v2.1.0, generated notes, three bundles/checksums and
  actual binary names, headers and version output. Unity Editor remains separate.

Run cargo fmt --check, cargo clippy --all-targets -- -D warnings and cargo test
--locked. Use native ubuntu, macOS and Windows CI; build release artifacts there.
Test supported minimum Rust separately if a rust-version is declared.
The release workflow runs these checks on v* tag pushes; ordinary commits and PRs
no longer trigger automatic builds. Before tagging, run the relevant checks locally.

Release-specific verification uses:

```sh
python -m unittest discover -s scripts -p 'test_release.py' -v
python scripts/release.py metadata --tag v2.0.0
```

It covers stable/build-metadata tags, standard and legacy dotted prereleases,
invalid tags, Cargo/lock mismatch, native archive contents/executable mode,
checksums, corruption and incomplete platform assets. After native builds, the
workflow checks the actual compiled binary version before bundling and gates
publication on all native and minimum-toolchain jobs. See RELEASING.md.

Fixtures are generated with explicit entry order and payload bytes. Regression
coverage includes text/binary/empty assets, folders, missing metas, non-Assets
paths, BOM/CRLF pathname, ./ names, icons/manifests/unknown entries, GNU/PAX and
long names, malformed gzip/tar, conflicting GUIDs and normalized paths.
An independent Python tarfile smoke check validates the Rust output container.

Exercise the CLI end to end: selection, filtering, raw bytes/stdout separation,
JSON and exit codes; extraction collision/no-overwrite/symlink guards; repack
and mutation preservation including unknown siblings; manifest/icon edits;
UPM packages with Runtime/Editor/Tests/Samples~/Documentation~, existing GUIDs,
generated metas, exclusions, both layouts and dependency warnings. Test that
failed mutations leave source and existing destination untouched.

Read-only commands must succeed when TMPDIR/TEMP/TMP point to nonexistent paths.
Large payload scans must not buffer payloads into the index.

## Unity import gate (separate, requires Editor)

Test in clean Unity 2022.3 LTS and Unity 6 projects:

1. Import generated standard packages; confirm a single resource per GUID and
   correct asset references, importer settings and empty folders.
2. Check root .icon.png in the import UI; confirm dependencies from the package
   manager manifest. Root .cover.png is legacy preservation, not a known UI icon.
3. Import Packages/name output; verify package.json, assemblies and samples.
4. Compare original vs repacked packages and verify only deliberate edits differ.
5. Test non-Assets content that each Editor accepts, documenting rejected paths.

Do not label Unity import validation complete based on CLI round trips. In cloud
environments without an Editor, record this gate as NOT RUN.

## Recorded implementation validation

v2.1.0 development checks: local Linux format/strict clippy and 43 integration
regressions PASS. Independent Python USTAR/GNU/local-PAX checks now also cover
scope/statistics, metadata dump/edit and untouched bytes. Thirty-two distinct
literal new-feature documentation commands and two SKILL Python examples PASS
on generated fixtures; frontmatter and local documentation links were checked.
Seven release-script tests PASS. Native v2.1.0 CI and publication are still pending;
PROGRESS.md records the active release gate and subsequent outcome.

Release commit 8a07187296c64d67f904825d371ed59375e03828 (v2.0.0) passed
[release CI](https://github.com/CKylinMC/ununitypackage/actions/runs/36753329704)
on Linux, macOS and Windows, and the Rust 1.88 minimum-toolchain job. Native jobs
run format checking, strict clippy, regression tests, release builds and the
independent Python USTAR/GNU/local-PAX interoperability script. Linux has 32
integration regressions; platform-specific cases run on the corresponding hosts.

Version validation and the publication job also passed. The published
[v2.0.0 Release](https://github.com/CKylinMC/ununitypackage/releases/tag/v2.0.0)
is stable (prerelease=false, draft=false), with generated changelog notes, three
native bundles and their .sha256 companions. All six assets were downloaded:
checksums, file sets, root binary names and native binary architectures matched.
Unix executable modes were 0755, and the downloaded Linux uup reports 2.0.0.
Each bundle includes README, SKILL.md, LICENSE, USAGE.md and RELEASING.md.

Earlier offline installation and 40 usage workflow checks established command
behavior; the additional Agent skill examples passed 41 literal CLI invocations
and a Python JSON/bytes example. Intel macOS has not had native-runner validation.
Unity Editor import remains NOT RUN.
