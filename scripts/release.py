"""Validate release tags, bundle native binaries, and verify release checksums."""
import argparse
import hashlib
import json
import re
import stat
import subprocess
import tarfile
import tomllib
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
NUMBER = r"(?:0|[1-9][0-9]*)"
IDENTIFIER = r"(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)"
SEMVER = re.compile(
    rf"(?P<base>{NUMBER}\.{NUMBER}\.{NUMBER})"
    rf"(?:-(?P<prerelease>{IDENTIFIER}(?:\.{IDENTIFIER})*))?"
    r"(?:\+(?P<build>[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
)
LEGACY = re.compile(
    rf"(?P<base>{NUMBER}\.{NUMBER}\.{NUMBER})"
    r"\.(?P<label>alpha|beta|rc)(?:[-.](?P<number>[0-9]+))?"
    r"(?P<build>\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?"
)
TARGETS = (("linux", "x86_64"), ("macos", "aarch64"), ("windows", "x86_64"))
DOCUMENTS = ("README.md", "SKILL.md", "LICENSE", "docs/USAGE.md", "docs/RELEASING.md")


def parse_tag(tag):
    if not tag.startswith("v"):
        raise ValueError("release tag must start with v")
    version = tag[1:]
    match = SEMVER.fullmatch(version)
    if not match:
        legacy = LEGACY.fullmatch(version)
        if not legacy:
            raise ValueError(f"invalid release version: {tag}")
        version = legacy["base"] + "-" + legacy["label"]
        if legacy["number"] is not None:
            version += "." + legacy["number"]
        version += legacy["build"] or ""
        match = SEMVER.fullmatch(version)
    if not match:
        raise ValueError(f"invalid SemVer prerelease: {tag}")
    return {"tag": tag, "version": version, "prerelease": bool(match["prerelease"])}


def metadata(tag, root=ROOT):
    release = parse_tag(tag)
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    lock = tomllib.loads((root / "Cargo.lock").read_text())
    package = manifest["package"]
    versions = [entry["version"] for entry in lock["package"] if entry["name"] == package["name"] and "source" not in entry]
    if package["version"] != release["version"] or versions != [release["version"]]:
        raise ValueError("tag, Cargo.toml and Cargo.lock versions must agree; legacy dotted tags use their normalized SemVer version")
    return release


def asset_name(tag, platform, arch):
    parse_tag(tag)
    if (platform, arch) not in TARGETS:
        raise ValueError("unsupported release platform/architecture")
    extension = ".zip" if platform == "windows" else ".tar.gz"
    return f"uup-{tag}-{platform}-{arch}{extension}"


def checksum(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def create_bundle(root, binary, output, release, platform, arch):
    name = asset_name(release["tag"], platform, arch)
    executable = "uup.exe" if platform == "windows" else "uup"
    files = [(binary, executable, 0o755)] + [(root / name, name, 0o644) for name in DOCUMENTS]
    for source, _, _ in files:
        if not source.is_file() or source.is_symlink():
            raise ValueError(f"missing or linked release file: {source}")
    output.mkdir(parents=True, exist_ok=True)
    destination = output / name
    if platform == "windows":
        with zipfile.ZipFile(destination, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            for source, relative, mode in files:
                info = zipfile.ZipInfo(relative)
                info.create_system = 3
                info.external_attr = (stat.S_IFREG | mode) << 16
                info.compress_type = zipfile.ZIP_DEFLATED
                archive.writestr(info, source.read_bytes())
    else:
        with tarfile.open(destination, "w:gz", format=tarfile.PAX_FORMAT) as archive:
            for source, relative, mode in files:
                info = archive.gettarinfo(str(source), arcname=relative)
                info.mode, info.uid, info.gid, info.mtime = mode, 0, 0, 0
                info.uname = info.gname = ""
                with source.open("rb") as stream:
                    archive.addfile(info, stream)
    (output / (name + ".sha256")).write_text(f"{checksum(destination)}  {name}\n", encoding="utf-8")
    return destination


def verify_dist(tag, output):
    expected = {asset_name(tag, platform, arch) for platform, arch in TARGETS}
    expected |= {name + ".sha256" for name in expected}
    actual = {path.name for path in output.iterdir()}
    if actual != expected:
        raise ValueError(f"release assets differ: missing={sorted(expected - actual)}, unexpected={sorted(actual - expected)}")
    for platform, arch in TARGETS:
        name = asset_name(tag, platform, arch)
        archive = output / name
        digest = output / (name + ".sha256")
        if archive.is_symlink() or digest.is_symlink():
            raise ValueError("release assets must not be symlinks")
        if digest.read_text(encoding="utf-8") != f"{checksum(archive)}  {name}\n":
            raise ValueError(f"checksum mismatch: {name}")
    return sorted(expected)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for command in ("metadata", "bundle", "verify-dist"):
        subparser = commands.add_parser(command)
        subparser.add_argument("--tag", required=True)
        if command == "metadata":
            subparser.add_argument("--github-output", type=Path)
        else:
            subparser.add_argument("--output", type=Path, default=Path("dist"))
        if command == "bundle":
            subparser.add_argument("--platform", required=True)
            subparser.add_argument("--arch", required=True)
    args = parser.parse_args()
    try:
        release = metadata(args.tag)
        if args.command == "metadata":
            if args.github_output:
                with args.github_output.open("a", encoding="utf-8") as stream:
                    stream.write(f"version={release['version']}\nprerelease={str(release['prerelease']).lower()}\n")
            print(json.dumps(release))
        elif args.command == "bundle":
            binary = ROOT / "target/release" / ("uup.exe" if args.platform == "windows" else "uup")
            result = subprocess.run([str(binary), "--version"], capture_output=True, text=True, check=True)
            if result.stdout.strip() != f"uup {release['version']}":
                raise ValueError("compiled binary version does not match release")
            print(create_bundle(ROOT, binary, args.output, release, args.platform, args.arch))
        else:
            print(json.dumps(verify_dist(args.tag, args.output)))
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"error: {error}\n")


if __name__ == "__main__":
    main()
