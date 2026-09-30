# Verification strategy

Run cargo fmt --check, cargo clippy --all-targets -- -D warnings and cargo test
--locked. Use native ubuntu, macOS and Windows CI; build release artifacts there.
Test supported minimum Rust separately if a rust-version is declared.

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
