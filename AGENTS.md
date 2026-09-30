# Project instructions

The project/Cargo package is uup-cli; its binary is uup (uup.exe on Windows).
Work on next unless the user requests a different branch. Keep source, docs and
cloud progress checkpoints committed and pushed as described in docs/CLOUD-RUNBOOK.md.

## Version and release policy

The original .NET line is 1.0.0. The Rust line starts at 2.0.0.

When the user requests a release:

1. Use the exact version/tag and stable/prerelease status specified by the user.
2. Otherwise inspect changes since the previous release. Choose a patch for fixes,
   a minor for compatible features, and a major for breaking changes. If the
   intended scope or stable/prerelease status cannot be inferred, ask one concise
   clarification; do not ask again when the session already provides the answer.
3. Update Cargo.toml and the local uup-cli entry in Cargo.lock together. Synchronize
   version-dependent examples in README.md, SKILL.md and docs. Use valid SemVer in
   Cargo and prefer matching tags such as v2.0.1 or v2.1.0-beta.1.
4. Run release metadata validation, release-script tests and relevant Rust checks.
   Commit and push the release commit before creating an annotated version tag on
   that exact commit. Tag publication triggers the user-authorized automatic
   GitHub Release; do not add a separate approval step for an already requested release.
5. Never force-move published tags. Check existing local/remote tags and releases,
   verify the remote tag SHA and follow the Actions result. Re-run a failed release
   workflow on the same tag after transient failures; code changes need a new version.

Ordinary development/documentation work does not create release tags. Tag-triggered
Actions replace automatic per-commit builds. Release steps, legacy dotted tag
normalization and recovery are documented in docs/RELEASING.md.
