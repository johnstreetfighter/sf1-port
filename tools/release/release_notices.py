"""Collect each locked Rust dependency's license and notice files for a package.

The Linux packager uses this module. `package-windows.ps1` holds the same
rules in PowerShell; change both together.
"""

from __future__ import annotations

import base64
import json
from pathlib import Path
import re
import shutil
import subprocess
from urllib.parse import quote, urlparse
from urllib.request import urlopen

# Names match in any letter case, as PowerShell's -match does: hound ships `license`.
NOTICE_NAME = re.compile(r"^(LICENSE|LICENCE|COPYING|NOTICE|UNLICENSE|COPYRIGHT)([._-].*)?$", re.I)
ROOT_LICENSE_NAME = re.compile(r"^(LICENSE|LICENCE|COPYING|UNLICENSE)([._-].*)?$", re.I)
UPSTREAM_NAME = re.compile(r"^(LICENSES?|LICENCES?|COPYING|NOTICE|UNLICENSE|COPYRIGHT)([._-].*)?$", re.I)
LICENSE_FOLDERS = {"LICENSES", "LICENCES"}


def in_license_folder(parts: tuple[str, ...]) -> bool:
    return any(part.upper() in LICENSE_FOLDERS for part in parts)
INDEX_HEADER = [
    "# Locked Rust dependency notices",
    "",
    "The executable uses default sf-game features, including statically linked libopenmpt. Cargo.lock records "
    "exact package identities; the files here retain dependency license and notice texts. The complete shaders "
    "retain their own headers and license files. Source for this game is linked in BUILD.txt; registry dependency "
    "source is available at the package links below.",
    "",
]
# This published crate and its complete pinned tree were checked. Retain its
# actual MIT declaration and source; do not invent a notice.
AUDITED_OMISSION = ("vec_extract_if_polyfill", "0.1.0", "11267b12cc4ec3350026bd496654eb17746eaed6")
OMISSION_NOTE = (
    'The original Cargo.toml declares license = "MIT". This published crate and pinned upstream revision provide '
    "no separate license text, copyright notice or author field. The original manifests and complete src tree are "
    "retained verbatim in original-source. This records the upstream material; it does not add a copyright notice "
    "or new grant."
)


def gh_api(path: str):
    result = subprocess.run(["gh", "api", path], capture_output=True, text=True, encoding="utf-8", timeout=120)
    if result.returncode:
        raise ValueError(f"GitHub request failed for {path}: {result.stderr.strip()}")
    return json.loads(result.stdout)


def gitlab_api(url: str):
    with urlopen(url, timeout=30) as response:  # noqa: S310 - fixed https://gitlab.com prefix
        return json.load(response)


def local_notices(manifest_folder: Path) -> list[Path]:
    """The license and notice files a crate's own archive carries."""
    return sorted(
        path for path in manifest_folder.rglob("*")
        if path.is_file() and (NOTICE_NAME.match(path.name)
                               or in_license_folder(path.relative_to(manifest_folder).parts[:-1]))
    )


def has_own_license(manifest_folder: Path, notices: list[Path], license_file: str | None) -> bool:
    root = any(path.parent == manifest_folder and ROOT_LICENSE_NAME.match(path.name) for path in notices)
    folder = any(in_license_folder(path.relative_to(manifest_folder).parts[:1]) for path in notices)
    return root or folder or bool(license_file)


class Upstream:
    """Pinned upstream notices for crates whose archive omits them."""

    def __init__(self, api=gh_api, gitlab=gitlab_api):
        self.api = api
        self.gitlab = gitlab
        self.cache: dict[str, tuple[list[tuple[str, bytes]], str, bool]] = {}

    def notices(self, dependency: dict, manifest_folder: Path) -> tuple[list[tuple[str, bytes]], str, bool]:
        """Files as (path, bytes), their source line, and whether upstream has no notice."""
        name = dependency["name"]
        vcs_file = manifest_folder / ".cargo_vcs_info.json"
        if not vcs_file.is_file():
            raise ValueError(f"No license text or pinned upstream revision for {name}.")
        revision = json.loads(vcs_file.read_text(encoding="utf-8")).get("git", {}).get("sha1", "")
        if not re.fullmatch(r"[0-9a-f]{40}", revision):
            raise ValueError(f"Missing pinned upstream revision for {name}.")
        repository = dependency.get("repository") or ""
        url = urlparse(repository)
        repo_path = url.path.strip("/").replace(".git", "")
        key = f"{repository}@{revision}"
        if key in self.cache:
            return self.cache[key]
        source = f"{repository} (commit {revision})"
        if url.hostname == "github.com":
            def listing(folder: str):
                suffix = f"/{folder}" if folder else ""
                return self.api(f"repos/{repo_path}/contents{suffix}?ref={revision}")

            def content(path: str) -> bytes:
                return base64.b64decode(self.api(f"repos/{repo_path}/contents/{path}?ref={revision}")["content"])
        elif url.hostname == "gitlab.com":
            base = f"https://gitlab.com/api/v4/projects/{quote(repo_path, safe='')}/repository"

            def listing(folder: str):
                path = f"path={quote(folder, safe='')}&" if folder else ""
                return self.gitlab(f"{base}/tree?{path}ref={revision}&per_page=100")

            def content(path: str) -> bytes:
                return base64.b64decode(self.gitlab(f"{base}/files/{quote(path, safe='')}?ref={revision}")["content"])
        else:
            raise ValueError(f"No supported pinned license source for {name}.")

        root = [entry for entry in listing("") if UPSTREAM_NAME.match(entry["name"])]
        folders = [entry["path"] for entry in root if entry["type"] in ("dir", "tree")]
        files = [entry["path"] for entry in root if entry["type"] in ("file", "blob")]
        while folders:
            for child in listing(folders.pop()):
                if child["type"] in ("dir", "tree"):
                    folders.append(child["path"])
                elif child["type"] in ("file", "blob"):
                    files.append(child["path"])
        if not files:
            if (name, dependency["version"], revision) != AUDITED_OMISSION:
                raise ValueError(f"Pinned upstream has no root license files for {name}.")
            result = ([("UPSTREAM-NOTICE-STATUS.txt", OMISSION_NOTE.encode("utf-8"))], source, True)
        else:
            result = ([(path, content(path)) for path in files], source, False)
        self.cache[key] = result
        return result


def collect(packages: list[dict], package_root: Path, upstream: Upstream | None = None) -> int:
    """Write `licenses/` under `package_root` for every registry dependency."""
    upstream = upstream or Upstream()
    license_root = package_root / "licenses" / "rust-dependencies"
    license_root.mkdir(parents=True, exist_ok=True)
    index = list(INDEX_HEADER)
    dependencies = sorted((p for p in packages if p.get("source")), key=lambda p: (p["name"], p["version"]))
    for dependency in dependencies:
        label = f"{dependency['name']}-{dependency['version']}"
        folder = license_root / label
        folder.mkdir(parents=True, exist_ok=True)
        manifest_folder = Path(dependency["manifest_path"]).parent
        notices = local_notices(manifest_folder)
        for notice in notices:
            destination = folder / notice.relative_to(manifest_folder)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(notice, destination)
        license_file = dependency.get("license_file")
        if not has_own_license(manifest_folder, notices, license_file):
            files, source, missing = upstream.notices(dependency, manifest_folder)
            for name, data in files:
                destination = folder / name
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(data)
            (folder / "UPSTREAM-SOURCE.txt").write_text(source, encoding="utf-8")
            if missing:
                original = folder / "original-source"
                original.mkdir()
                for manifest in ("Cargo.toml", "Cargo.toml.orig"):
                    if (manifest_folder / manifest).is_file():
                        shutil.copyfile(manifest_folder / manifest, original / manifest)
                shutil.copytree(manifest_folder / "src", original / "src")
        if license_file:
            explicit = manifest_folder / license_file
            if explicit.is_file():
                shutil.copyfile(explicit, folder / explicit.name)
        index.append(f"- {label}; license: {dependency.get('license')}; "
                     f"source: https://crates.io/crates/{dependency['name']}/{dependency['version']}")
    (package_root / "licenses" / "README.md").write_text("\n".join(index) + "\n", encoding="utf-8")
    return len(dependencies)
