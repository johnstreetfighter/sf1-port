"""Locate the game checkout used by historical ROM analysis tools."""
from os import environ
from pathlib import Path


def reference_folder():
    root = environ.get("SF1_PORT_ROOT")
    if root:
        return Path(root).expanduser() / "tools" / "reference"

    pack = Path(__file__).resolve().parents[1]
    for candidate in (*pack.parents, pack.parent / "sf1-port"):
        reference = candidate / "tools" / "reference"
        if (reference / "rom-manifest.json").is_file():
            return reference
    raise FileNotFoundError("Set SF1_PORT_ROOT to the game checkout with the local ROM set.")


REFERENCE = reference_folder()
MAN = REFERENCE / "rom-manifest.json"
ROMS = REFERENCE / "local" / "roms" / "sf"
