#!/bin/sh
# Build the locked Linux release executable inside the runtime baseline.
#
# Run as root in a fresh ubuntu:22.04 container with the tagged checkout
# mounted at /source and an empty build folder at /build:
#
#   docker run --rm -e RUSTUP_TOOLCHAIN=1.96.0 \
#     -v "$PWD":/source -v /path/to/isolated-target:/build \
#     ubuntu:22.04 sh /source/tools/release/build-linux.sh
#
# The executable is /build/release/sf-game. Linking against Ubuntu 22.04's
# glibc 2.35 is what sets the oldest system the package runs on.
set -eu

: "${RUSTUP_TOOLCHAIN:?set RUSTUP_TOOLCHAIN to the release Rust version}"
[ -f /source/Cargo.lock ] || { echo "mount the tagged checkout at /source" >&2; exit 1; }
[ -d /build ] || { echo "mount an empty build folder at /build" >&2; exit 1; }

export DEBIAN_FRONTEND=noninteractive
apt-get update -q
# The compilers, plus headers for the libraries the game links or loads:
# ALSA (sound), udev (gamepads), X11 and Wayland (the window).
apt-get install -y -q --no-install-recommends \
    ca-certificates curl git build-essential pkg-config \
    libasound2-dev libudev-dev \
    libx11-dev libxcursor-dev libxkbcommon-dev libwayland-dev

export CARGO_HOME=/cargo RUSTUP_HOME=/rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path --profile minimal --default-toolchain "$RUSTUP_TOOLCHAIN"
export PATH="/cargo/bin:$PATH"

# The checkout belongs to the host user; the build script reads its revision.
git config --global --add safe.directory /source

# Keep container paths out of the executable.
maps="/source=source /build=build /cargo=cargo /rustup=toolchain"
rustflags=""
cflags=""
for map in $maps; do
    rustflags="$rustflags --remap-path-prefix=$map"
    cflags="$cflags -ffile-prefix-map=$map"
done
export RUSTFLAGS="${rustflags# }"
export CFLAGS="${cflags# }" CXXFLAGS="${cflags# }"

cd /source
cargo build --release --locked --target-dir /build -p sf-game --bin sf-game
# The host user packages and verifies what root built here.
chmod -R a+rX /build
