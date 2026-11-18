"""Package an already-built, exact-tag Linux x86_64 release as a tar.gz with a SHA-256 sidecar."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import gzip
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile

import release_notices
from release_packs import require, stage

TARGET = "x86_64-unknown-linux-gnu"
BASELINE = "Ubuntu 22.04 (glibc 2.35)"
SOURCE_FILES = ("README.md", "THIRD_PARTY_NOTICES.md", "Cargo.lock")
EXECUTABLE = "sf-game"


def run(source: Path, *command: str, env: dict[str, str] | None = None) -> str:
    return subprocess.run(command, cwd=source, env=env, check=True, capture_output=True,
                          text=True, encoding="utf-8", timeout=300).stdout.strip()


def package_name(tag: str) -> str:
    return f"sf1-port-{tag}-linux-x86_64"


def quick_start(tag: str) -> list[str]:
    return [
        f"Street Fighter: The World Tournament - {tag} (Linux x86_64)",
        "",
        "1. Extract the whole archive anywhere in your home folder:",
        f"     tar -xzf {package_name(tag)}.tar.gz",
        "   Keep the shaders and assets folders beside sf-game.",
        f"2. Use 64-bit desktop Linux at least as new as {BASELINE}, or SteamOS 3,",
        "   with a Vulkan graphics driver, ALSA sound (PipeWire and PulseAudio provide it) and X11 or Wayland.",
        "   No development packages, root access or system changes are needed.",
        "3. Put your legally obtained US set 1 six-button sf ROM files in one of these folders:",
        "     ~/.local/share/sf1-port/roms/sf   (kept when you replace the package)",
        "     roms/sf beside sf-game",
        "   Keep the original ROM filenames. A folder, sf.zip or sf.7z elsewhere also works:",
        "     ./sf-game --roms /path/to/sf.zip",
        "4. Run ./sf-game from a terminal or your file manager. It can be started from any folder.",
        "",
        "The playable art, voice, and music packs are included under assets. Each pack has its own README.",
        "Settings, saves and sf-game.log are kept in ~/.local/share/sf1-port",
        "($XDG_DATA_HOME/sf1-port when your desktop sets that variable).",
        "To update, extract the new archive and delete the old folder; that folder is not touched.",
        "See README.md for controls and features.",
        "",
        "BUILD.txt records the exact source commit and executable checksum. licenses and THIRD_PARTY_NOTICES.md "
        "retain dependency notices.",
        "Verify the release SHA-256 against the downloaded archive:",
        f"     sha256sum -c {package_name(tag)}.tar.gz.sha256",
    ]


def build_info(tag: str, commit: str, banner: str, compiler: str, exe_digest: str, packaged: str) -> list[str]:
    return [
        f"Street Fighter: The World Tournament - {tag} Linux x86_64 package",
        f"Tag: {tag}",
        f"Source commit: {commit}",
        f"Source: https://github.com/johnstreetfighter/sf1-port/tree/{commit}",
        f"Target: {TARGET}",
        f"Runtime baseline: {BASELINE}",
        "Profile: release; default features (tracker/libopenmpt included statically)",
        f"Executable version: {banner}",
        f"Compiler: {compiler}",
        f"Packaged (UTC): {packaged}",
        f"{EXECUTABLE} SHA-256: {exe_digest}",
        "",
        "Built from the tagged source with the playable art, voice, and music packs.",
        "ROMs, authoring archives, settings and logs are excluded.",
        "This package does not grant new licenses; existing source/dependency notices are retained.",
    ]


def write_archive(package: Path, archive: Path) -> None:
    """A tar.gz of `package` with root ownership, fixed modes and sorted members."""
    def member(path: Path) -> tarfile.TarInfo:
        info = tarfile.TarInfo(path.relative_to(package.parent).as_posix())
        info.uid = info.gid = 0
        info.uname = info.gname = "root"
        info.mtime = int(max(path.stat().st_mtime, 315532800))  # no earlier than 1980
        if path.is_dir():
            info.type = tarfile.DIRTYPE
            info.mode = 0o755
        else:
            info.size = path.stat().st_size
            info.mode = 0o755 if path == package / EXECUTABLE else 0o644
        return info

    with archive.open("xb") as raw, gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as packed, \
            tarfile.open(fileobj=packed, mode="w", format=tarfile.PAX_FORMAT) as tar:
        tar.addfile(member(package))
        for path in sorted(package.rglob("*"), key=lambda p: p.relative_to(package).as_posix()):
            require(path.is_dir() or (path.is_file() and not path.is_symlink()),
                    f"staged package entry is not a regular file: {path}")
            if path.is_dir():
                tar.addfile(member(path))
            else:
                with path.open("rb") as stream:
                    tar.addfile(member(path), stream)


def package(tag: str, build: Path, output: Path, source: Path) -> tuple[Path, Path]:
    exe = build / EXECUTABLE
    require(exe.is_file(), f"the build directory must contain {EXECUTABLE}")
    head = run(source, "git", "rev-parse", "HEAD")
    require(run(source, "git", "rev-parse", f"{tag}^{{commit}}") == head, "source HEAD must equal the release tag")
    require(subprocess.run(["git", "diff", "--quiet", "HEAD", "--"], cwd=source).returncode == 0,
            "commit tracked source changes before packaging")
    # Only metadata is read here; the build completed separately.
    metadata = json.loads(run(source, "cargo", "metadata", "--locked", "--format-version", "1",
                              "--filter-platform", TARGET,
                              env=os.environ | {"CARGO_TARGET_DIR": str(build.parent)}))
    banner = subprocess.run([str(exe), "--version"], check=True, capture_output=True, text=True,
                            encoding="utf-8", timeout=30).stdout.strip()
    require(tag in banner and head[:8] in banner,
            "the executable's --version must identify both the requested version and the exact source revision")
    compiler = run(source, "rustc", "--version")

    output.mkdir(parents=True, exist_ok=True)
    name = package_name(tag)
    staged = output / name
    archive = output / f"{name}.tar.gz"
    require(not staged.exists(), f"package directory already exists: {staged}")
    require(not archive.exists(), f"package archive already exists: {archive}")
    staged.mkdir()
    shutil.copyfile(exe, staged / EXECUTABLE)
    shutil.copytree(source / "shaders", staged / "shaders")
    stage(source, head, staged)
    for name_ in SOURCE_FILES:
        shutil.copyfile(source / name_, staged / name_)
    if (source / "docs" / "netplay.md").is_file():
        (staged / "docs").mkdir()
        shutil.copyfile(source / "docs" / "netplay.md", staged / "docs" / "netplay.md")
    release_notices.collect(metadata["packages"], staged)

    exe_digest = hashlib.sha256((staged / EXECUTABLE).read_bytes()).hexdigest()
    packaged = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    (staged / "BUILD.txt").write_text(
        "\n".join(build_info(tag, head, banner, compiler, exe_digest, packaged)) + "\n", encoding="utf-8")
    (staged / "QUICKSTART.txt").write_text("\n".join(quick_start(tag)) + "\n", encoding="utf-8")

    write_archive(staged, archive)
    sidecar = archive.with_name(archive.name + ".sha256")
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    sidecar.write_bytes(f"{digest}  {archive.name}\n".encode("ascii"))
    return archive, sidecar


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--build-directory", required=True, type=Path)
    parser.add_argument("--output-directory", required=True, type=Path)
    parser.add_argument("--source-root", type=Path, default=Path(__file__).resolve().parents[2])
    args = parser.parse_args()
    if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+", args.tag):
        parser.error("expected a vX.Y.Z release tag")
    try:
        for path in package(args.tag, args.build_directory.resolve(), args.output_directory.resolve(),
                            args.source_root.resolve()):
            print(path)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        detail = getattr(error, "stderr", "") or ""
        print(f"Could not package the Linux release: {error} {detail}".rstrip(), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
