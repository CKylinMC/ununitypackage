# Verification strategy

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
