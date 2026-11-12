"""Build the locked Windows release executable in a dedicated Cargo target directory."""

import argparse
import os
from pathlib import Path
import shlex
import shutil
import subprocess


def path_spellings(path: Path, *, extended: bool) -> set[str]:
    prefixes = set()
    for spelling in {str(path.absolute()), str(path.resolve())}:
        if spelling.upper().startswith("\\\\?\\UNC\\"):
            spelling = "\\\\" + spelling[8:]
        else:
            spelling = spelling.removeprefix("\\\\?\\")
        prefixes.add(spelling)
        if extended:
            if spelling.startswith("\\\\"):
                prefixes.add("\\\\?\\UNC\\" + spelling[2:])
            else:
                prefixes.add("\\\\?\\" + spelling)

    variants = set()
    for prefix in prefixes:
        slash = prefix.replace("\\", "/")
        variants.update((prefix, slash, prefix.lower(), slash.lower()))
    return variants


def remaps(roots: list[tuple[Path, str]], *, extended: bool) -> list[tuple[str, str]]:
    mappings = {
        spelling: name
        for path, name in roots
        for spelling in path_spellings(path, extended=extended)
    }
    return sorted(mappings.items(), key=lambda item: (len(item[0]), item[0]))


def build_environment(source: Path, target: Path) -> dict[str, str]:
    env = os.environ.copy()
    profile = Path.home()
    cargo_home = source / Path(env.get("CARGO_HOME") or profile / ".cargo")
    rustup_home = source / Path(env.get("RUSTUP_HOME") or profile / ".rustup")
    roots = [
        (source / env[variable], name)
        for variable, name in (("GITHUB_WORKSPACE", "workspace"), ("RUNNER_TEMP", "temp"))
        if env.get(variable)
    ]
    roots.extend([
        (profile, "profile"),
        (cargo_home, "cargo"),
        (rustup_home, "toolchain"),
        (source, "source"),
        (target, "build"),
    ])
    tool_roots = [
        (source / env[variable], "toolchain")
        for variable in ("ProgramFiles", "ProgramFiles(x86)", "ProgramData")
        if env.get(variable)
    ]

    # rustc uses the last matching prefix. MSVC uses the first one.
    rust_flags = [
        f"--remap-path-prefix={path}={name}"
        for path, name in remaps(tool_roots + roots, extended=True)
    ]
    rust_flags.extend(["-C", "link-arg=/PDBALTPATH:%_PDB%"])
    native_flags = ["/experimental:deterministic"]
    native_flags.extend(
        f"/pathmap:{path}={name}"
        for path, name in reversed(remaps(roots, extended=False))
    )

    env["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(rust_flags)
    # The locked cc crate reads these variables with shlex when this option is set.
    env["CC_SHELL_ESCAPED_FLAGS"] = "1"
    env["CFLAGS"] = env["CXXFLAGS"] = shlex.join(native_flags)
    return env


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target_dir", type=Path, help="dedicated Cargo target directory")
    args = parser.parse_args()
    if os.name != "nt":
        parser.error("this build requires Windows and an MSVC Rust toolchain")

    source = Path(__file__).resolve().parents[2]
    target = args.target_dir.resolve()
    rtk = shutil.which("rtk")
    command = [rtk, "proxy", "cargo"] if rtk else ["cargo"]
    command.extend([
        "build", "--release", "--locked", "--target-dir", str(target),
        "-p", "sf-game", "--bin", "sf-game",
    ])
    return subprocess.run(command, cwd=source, env=build_environment(source, target)).returncode


if __name__ == "__main__":
    raise SystemExit(main())
