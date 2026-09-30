"""Release classification, version alignment, and distributable regression checks."""
import tempfile
import tarfile
import unittest
import zipfile
from pathlib import Path

import release


class ReleaseTests(unittest.TestCase):
    def test_stable_and_build_metadata_are_not_prereleases(self):
        for tag in ("v2.0.0", "v2.1.3", "v2.0.0+beta", "v2.0.0+build.1"):
            with self.subTest(tag=tag):
                self.assertFalse(release.parse_tag(tag)["prerelease"])

    def test_semver_and_legacy_prereleases(self):
        cases = {
            "v2.0.0-beta.1": "2.0.0-beta.1",
            "v2.0.0-alpha": "2.0.0-alpha",
            "v2.0.0-rc.1+build.2": "2.0.0-rc.1+build.2",
            "v2.0.0-experimental": "2.0.0-experimental",
            "v2.0.0.beta": "2.0.0-beta",
            "v2.0.0.alpha": "2.0.0-alpha",
            "v2.0.0.beta-1": "2.0.0-beta.1",
            "v2.0.0.alpha.2": "2.0.0-alpha.2",
        }
        for tag, version in cases.items():
            with self.subTest(tag=tag):
                result = release.parse_tag(tag)
                self.assertTrue(result["prerelease"])
                self.assertEqual(result["version"], version)

    def test_invalid_versions_and_unsafe_tag_paths_fail(self):
        for tag in ("2.0.0", "v2", "v02.0.0", "v2.0.0-beta.01", "v2.0.0.beta-01", "v2.0.0.", "v2.0.0/other", "v2.0.0\n", "v2.0.0;echo nope"):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                release.parse_tag(tag)

    def test_manifest_and_lock_must_both_match_normalized_tag(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "Cargo.toml").write_text('[package]\nname="uup-cli"\nversion="2.0.0-beta.1"\n')
            (root / "Cargo.lock").write_text('[[package]]\nname="uup-cli"\nversion="2.0.0-beta.1"\n')
            self.assertTrue(release.metadata("v2.0.0.beta-1", root)["prerelease"])
            with self.assertRaises(ValueError):
                release.metadata("v2.0.0", root)
            (root / "Cargo.lock").write_text('[[package]]\nname="uup-cli"\nversion="2.0.0"\n')
            with self.assertRaises(ValueError):
                release.metadata("v2.0.0.beta-1", root)

    def test_native_archives_have_root_binary_docs_and_valid_checksums(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for relative in release.DOCUMENTS:
                source = root / relative
                source.parent.mkdir(parents=True, exist_ok=True)
                source.write_bytes(b"documentation")
            binary = root / "compiled-binary"
            binary.write_bytes(b"binary payload")
            output = root / "dist"
            metadata = release.parse_tag("v2.0.0")
            for platform, arch in release.TARGETS:
                archive = release.create_bundle(root, binary, output, metadata, platform, arch)
                expected_binary = "uup.exe" if platform == "windows" else "uup"
                expected_files = {expected_binary, *release.DOCUMENTS}
                if platform == "windows":
                    with zipfile.ZipFile(archive) as container:
                        self.assertEqual(set(container.namelist()), expected_files)
                        self.assertEqual(container.read(expected_binary), b"binary payload")
                else:
                    with tarfile.open(archive, "r:gz") as container:
                        self.assertEqual(set(container.getnames()), expected_files)
                        self.assertEqual(container.extractfile(expected_binary).read(), b"binary payload")
                        self.assertEqual(container.getmember(expected_binary).mode, 0o755)
            self.assertEqual(len(release.verify_dist("v2.0.0", output)), 6)
            archive.write_bytes(archive.read_bytes() + b"tampered")
            with self.assertRaisesRegex(ValueError, "checksum mismatch"):
                release.verify_dist("v2.0.0", output)

    def test_incomplete_asset_set_cannot_be_published(self):
        with tempfile.TemporaryDirectory() as temporary:
            with self.assertRaisesRegex(ValueError, "missing"):
                release.verify_dist("v2.0.0", Path(temporary))

    def test_missing_documentation_and_unsupported_architecture_fail(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            binary = root / "binary"
            binary.write_bytes(b"payload")
            with self.assertRaisesRegex(ValueError, "missing"):
                release.create_bundle(root, binary, root / "dist", release.parse_tag("v2.0.0"), "linux", "x86_64")
            with self.assertRaises(ValueError):
                release.asset_name("v2.0.0", "windows", "unknown")


if __name__ == "__main__":
    unittest.main()
