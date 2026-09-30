"""Independent Python tarfile -> Rust CLI -> Python tarfile interoperability."""
import io
import json
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path


def main():
    binary = Path(sys.argv[1]).resolve()

    def run(*args):
        result = subprocess.run([str(binary), *map(str, args)], capture_output=True, check=True)
        return result.stdout

    with tempfile.TemporaryDirectory(prefix="uup-interop-") as temporary:
        root = Path(temporary)
        for label, format_ in [("ustar", tarfile.USTAR_FORMAT), ("gnu", tarfile.GNU_FORMAT), ("pax", tarfile.PAX_FORMAT)]:
            guid = "0123456789abcdef0123456789abcdef"
            entries = {
                f"./{guid}/asset": bytes(range(256)) * 100,
                f"./{guid}/pathname": "\ufeffPackageSettings/中文.bin\r\n00".encode(),
                f"./{guid}/asset.meta": f"fileFormatVersion: 2\nguid: {guid}\n".encode(),
                "packagemanagermanifest/asset": b'{"dependencies":{},"custom":true}',
                "other/opaque.bin": b"unknown content",
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
            print(f"PASS: independent {label} read/repack/extract, binary bytes and unknown content")


if __name__ == "__main__":
    main()
