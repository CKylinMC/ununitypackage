"""Independent Python tarfile -> Rust CLI -> Python tarfile interoperability."""
import io
import json
import struct
import subprocess
import sys
import tarfile
import tempfile
import zlib
from pathlib import Path


def png():
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(b"\0\xff\0\0\xff")) + chunk(b"IEND", b""))


def main():
    binary = Path(sys.argv[1]).resolve()

    def run(*args):
        result = subprocess.run([str(binary), *map(str, args)], capture_output=True, check=True)
        return result.stdout

    with tempfile.TemporaryDirectory(prefix="uup-interop-") as temporary:
        root = Path(temporary)
        for label, format_ in [("ustar", tarfile.USTAR_FORMAT), ("gnu", tarfile.GNU_FORMAT), ("pax", tarfile.PAX_FORMAT)]:
            guid = "0123456789abcdef0123456789abcdef"
            upm_guid = "11111111111111111111111111111111"
            anim_guid = "22222222222222222222222222222222"
            entries = {
                f"./{guid}/asset": bytes(range(256)) * 100,
                f"./{guid}/pathname": "\ufeffPackageSettings/中文.bin\r\n00".encode(),
                f"./{guid}/asset.meta": f"fileFormatVersion: 2\nguid: {guid}\n".encode(),
                "packagemanagermanifest/asset": b'{"dependencies":{},"custom":true}',
                "other/opaque.bin": b"unknown content",
                f"./{upm_guid}/pathname": b"Packages/com.example.tool/package.json",
                f"./{upm_guid}/asset": b'{"name":"com.example.tool","version":"1.0.0","custom":{"keep":true}}',
                f"./{upm_guid}/asset.meta": f"fileFormatVersion: 2\nguid: {upm_guid}\n".encode(),
                f"./{anim_guid}/pathname": b"Assets/Demo/run.ANIM",
                f"./{anim_guid}/asset": b"animation bytes",
                f"./{anim_guid}/asset.meta": f"fileFormatVersion: 2\nguid: {anim_guid}\n".encode(),
                ".icon.png": png(),
                "UserSettings/preferences.json": b'{"theme":"dark"}',
            }
            if format_ != tarfile.USTAR_FORMAT:
                entries["other/" + "long-name/" * 40 + "data"] = b"long physical tar name"
            source = root / f"{label}.unitypackage"
            with tarfile.open(source, "w:gz", format=format_) as archive:
                for name, content in entries.items():
                    info = tarfile.TarInfo(name)
                    info.size = len(content)
                    info.mode = 0o640
                    if format_ == tarfile.PAX_FORMAT:
                        info.pax_headers = {"vendor.unity-test": "preserve-me"}
                    archive.addfile(info, io.BytesIO(content))
            assert json.loads(run("verify", source, "--json"))["valid"]
            assert run("cat", source, "--path", "PackageSettings/中文.bin") == bytes(range(256)) * 100
            assert run("ls", source, "Assets/Demo", "--json") == run("list", source, "--path", "Assets/Demo", "--json")
            # Search remains case-sensitive; statistics fold extension case.
            found = json.loads(run("find", source, "*.ANIM", "Assets/Demo", "--glob", "--json"))
            assert len(found) == 1 and found[0]["guid"] == anim_guid
            stats = json.loads(run("info", source, "Assets/Demo", "--json"))["statistics"]
            assert stats["files"]["count"] == 1 and stats["extensions"][".anim"]["count"] == 1
            summary = json.loads(run("metadata", source, "--json"))
            assert summary["counts"]["package-json"] == 1 and summary["counts"]["settings"] == 2
            meta_output = root / f"{label}-metadata"
            run("metadata", source, "dump", "-o", meta_output)
            assert (meta_output / "Packages/com.example.tool/package.json").read_bytes() == entries[f"./{upm_guid}/asset"]
            assert (meta_output / ".icon.png").read_bytes() == entries[".icon.png"]
            assert (meta_output / "PackageSettings/中文.bin").read_bytes() == entries[f"./{guid}/asset"]
            edited = root / f"{label}-edited.unitypackage"
            run("metadata", source, "edit", "package-json", "--set", '/description="Updated description"', "-o", edited)
            with tarfile.open(edited, "r:gz") as archive:
                modified = {entry.name: archive.extractfile(entry).read() for entry in archive if entry.isfile()}
            payload = json.loads(modified.pop(f"./{upm_guid}/asset"))
            assert payload["description"] == "Updated description" and payload["custom"]["keep"] is True
            untouched = {name: data for name, data in entries.items() if name != f"./{upm_guid}/asset"}
            assert modified == untouched, (label, "metadata edit changed unrelated content")
            destination = root / f"{label}-repacked.unitypackage"
            run("repack", source, "-o", destination)
            with tarfile.open(destination, "r:gz") as archive:
                actual = {entry.name: archive.extractfile(entry).read() for entry in archive if entry.isfile()}
                assert actual == entries, (label, "payload/name preservation failed")
            with tarfile.open(destination, "r:gz") as archive:
                for entry in archive:
                    assert entry.mode == 0o640
                    if format_ == tarfile.PAX_FORMAT:
                        assert entry.pax_headers["vendor.unity-test"] == "preserve-me"
            output = root / f"{label}-extracted"
            run("extract", source, "-o", output)
            assert (output / "PackageSettings/中文.bin").read_bytes() == bytes(range(256)) * 100
            assert (output / "packagemanagermanifest/asset").read_bytes() == entries["packagemanagermanifest/asset"]
            scoped_output = root / f"{label}-scoped"
            run("extract", source, "Assets/Demo", "-o", scoped_output)
            assert (scoped_output / "Assets/Demo/run.ANIM").read_bytes() == b"animation bytes"
            assert (scoped_output / "Assets/Demo/run.ANIM.meta").read_bytes() == entries[f"./{anim_guid}/asset.meta"]
            assert not (scoped_output / "Packages").exists()
            print(f"PASS: independent {label} read/repack/extract, scopes/stats, metadata dump/edit and unknown bytes")


if __name__ == "__main__":
    main()
