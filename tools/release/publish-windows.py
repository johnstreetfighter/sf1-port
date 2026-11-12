"""Publish a verified Windows package and check the GitHub downloads."""

import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
from tempfile import TemporaryDirectory
from zipfile import ZipFile

AUTOMATED_DRAFT = "<!-- sf1-port automated Windows release -->"


def gh(*args, check=True):
    rtk = shutil.which("rtk")
    command = [rtk, "proxy", "gh"] if rtk else ["gh"]
    result = subprocess.run(
        command + list(args), capture_output=True, text=True, encoding="utf-8",
        timeout=180, check=False,
    )
    if check and result.returncode:
        raise SystemExit(result.stderr.strip() or result.stdout.strip() or "GitHub CLI failed")
    return result


def release_view(repo, tag):
    result = gh(
        "release", "view", tag, "--repo", repo, "--json",
        "tagName,isDraft,isPrerelease,assets,url,body", check=False,
    )
    if result.returncode == 0:
        return json.loads(result.stdout)
    if result.stderr.strip().lower() == "release not found":
        return None
    raise SystemExit(result.stderr.strip() or "Could not inspect GitHub release")


def verify_remote_tag(repo, tag, commit):
    ref = json.loads(gh("api", f"repos/{repo}/git/ref/tags/{tag}").stdout)["object"]
    if ref["type"] != "tag":
        raise SystemExit("GitHub release tag must be annotated")
    tagged = json.loads(gh("api", f"repos/{repo}/git/tags/{ref['sha']}").stdout)["object"]
    if tagged["type"] != "commit" or tagged["sha"] != commit:
        raise SystemExit("GitHub release tag no longer points to the built commit")


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def package_files(archive, tag, commit):
    package = f"sf1-port-{tag}-windows-x86_64"
    if archive.name != f"{package}.zip":
        raise SystemExit(f"Expected archive named {package}.zip")
    sidecar = archive.with_suffix(".zip.sha256")
    archive_hash = sha256(archive)
    if sidecar.read_text(encoding="utf-8").strip() != f"{archive_hash}  {archive.name}":
        raise SystemExit("SHA-256 sidecar does not match the ZIP")
    with ZipFile(archive) as zipped:
        manifest = zipped.read(f"{package}/BUILD.txt").decode("utf-8-sig").splitlines()
    if f"Tag: {tag}" not in manifest or f"Source commit: {commit}" not in manifest:
        raise SystemExit("ZIP build identity does not match the release tag and source commit")
    return {archive: archive_hash, sidecar: sha256(sidecar)}


def assets_by_name(release):
    return {asset["name"]: asset for asset in release["assets"]}


def asset_matches(asset, path, digest):
    return (
        asset is not None
        and asset["state"] == "uploaded"
        and asset["size"] == path.stat().st_size
        and asset.get("digest") == f"sha256:{digest}"
    )


def upload_draft_files(repo, tag, release, files):
    assets = assets_by_name(release)
    for path, digest in files.items():
        existing = assets.get(path.name)
        if asset_matches(existing, path, digest):
            continue
        replace = []
        if existing:
            if AUTOMATED_DRAFT not in (release.get("body") or ""):
                raise SystemExit(f"Draft already contains a different {path.name}")
            replace = ["--clobber"]
        gh("release", "upload", tag, str(path), "--repo", repo, *replace)

    uploaded = release_view(repo, tag)
    if uploaded is None or not uploaded["isDraft"]:
        raise SystemExit("Expected the release to stay in draft while verifying uploads")
    assets = assets_by_name(uploaded)
    for path, digest in files.items():
        if not asset_matches(assets.get(path.name), path, digest):
            raise SystemExit(f"GitHub upload digest or size differs for {path.name}")


def verify_downloads(repo, tag, commit, archive_name, release, expected=None):
    with TemporaryDirectory(prefix="sf1-release-download-") as folder:
        gh(
            "release", "download", tag, "--repo", repo, "--dir", folder,
            "--pattern", archive_name, "--pattern", f"{archive_name}.sha256",
        )
        downloaded = package_files(Path(folder) / archive_name, tag, commit)
        assets = assets_by_name(release)
        for path, digest in downloaded.items():
            if not asset_matches(assets.get(path.name), path, digest):
                raise SystemExit(f"GitHub download digest or size differs for {path.name}")
        if expected is not None:
            if {path.name: digest for path, digest in downloaded.items()} != expected:
                raise SystemExit("GitHub downloads differ from the verified upload files")
    print(f"Verified the published ZIP and SHA-256 sidecar at {release['url']}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("repo", help="GitHub owner/repository")
    parser.add_argument("tag")
    parser.add_argument("commit")
    parser.add_argument("archive", type=Path)
    args = parser.parse_args()
    if not re.fullmatch(r"v[0-9]+\.[0-9]+\.[0-9]+", args.tag):
        parser.error("Expected a stable release tag in vMAJOR.MINOR.PATCH form")
    if not re.fullmatch(r"[0-9a-f]{40}", args.commit):
        parser.error("Expected a full source commit SHA")

    files = package_files(args.archive, args.tag, args.commit)
    verify_remote_tag(args.repo, args.tag, args.commit)
    release = release_view(args.repo, args.tag)
    if release is None:
        notes = (
            "Extract the Windows ZIP and follow QUICKSTART.txt to supply your own ROM set. "
            "The package was built from the release tag and verified on Windows.\n\n"
            + AUTOMATED_DRAFT
        )
        gh(
            "release", "create", args.tag, "--repo", args.repo, "--verify-tag",
            "--draft", "--title", args.tag, "--generate-notes", "--notes", notes,
        )
        release = release_view(args.repo, args.tag)

    if release is None or release["tagName"] != args.tag or release["isPrerelease"]:
        raise SystemExit("Expected a stable GitHub release for the requested tag")

    # A rerun can build different ZIP bytes because the package records build time.
    # Keep an already-published release and verify its own downloadable assets.
    expected = None
    if release["isDraft"]:
        upload_draft_files(args.repo, args.tag, release, files)
        expected = {path.name: digest for path, digest in files.items()}
        verify_remote_tag(args.repo, args.tag, args.commit)
        gh("release", "edit", args.tag, "--repo", args.repo, "--verify-tag", "--draft=false", "--latest")
        release = release_view(args.repo, args.tag)

    if release is None or release["tagName"] != args.tag or release["isDraft"] or release["isPrerelease"]:
        raise SystemExit("Could not confirm the stable GitHub release is published")
    verify_downloads(args.repo, args.tag, args.commit, args.archive.name, release, expected)


if __name__ == "__main__":
    main()
