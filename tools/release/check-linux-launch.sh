#!/bin/sh
# Launch the packaged Linux build on a clean baseline system as an ordinary user.
#
# Run as root in a fresh ubuntu:22.04 container with the folder holding the
# archive and its sidecar mounted read-only at /package:
#
#   docker run --rm -v /path/to/packages:/package:ro -v "$PWD/tools/release":/tools:ro \
#     ubuntu:22.04 sh /tools/check-linux-launch.sh
#
# Only runtime packages a desktop already has are installed: no compilers and
# no development packages. The window opens on a virtual X display with Mesa's
# software Vulkan driver, so this checks that the game starts and keeps its
# files; it says nothing about real graphics drivers, sound or controllers.
set -eu

if [ "${1:-}" != "--as-player" ]; then
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -q
    apt-get install -y -q --no-install-recommends \
        libasound2 libudev1 libvulkan1 mesa-vulkan-drivers \
        libx11-6 libx11-xcb1 libxcursor1 libxi6 libxrandr2 libxkbcommon0 libxkbcommon-x11-0 \
        xvfb xauth
    if dpkg-query -W -f '${Package}\n' | grep -E -- '-dev$'; then
        echo "FAIL development packages are installed" >&2
        exit 1
    fi
    for tool in cc gcc g++ clang rustc cargo pkg-config; do
        if command -v "$tool" > /dev/null; then
            echo "FAIL $tool is installed" >&2
            exit 1
        fi
    done
    echo "ok   no compilers or development packages are installed"
    useradd --create-home player
    cp "$0" /tmp/check-linux-launch.sh
    chmod a+rx /tmp/check-linux-launch.sh
    exec su player -c "sh /tmp/check-linux-launch.sh --as-player"
fi

fail() { echo "FAIL $*" >&2; exit 1; }

archive=$(ls /package/sf1-port-v*-linux-x86_64.tar.gz)
name=$(basename "$archive" .tar.gz)
(cd /package && sha256sum -c "$name.tar.gz.sha256") || fail "the archive does not match its SHA-256 sidecar"
echo "ok   the archive matches its SHA-256 sidecar"

data="$HOME/.local/share/sf1-port"
[ ! -e "$data" ] || fail "the player's folder exists before the first launch"

# Hold the window open for a while on a virtual display; a game that keeps
# running until the timeout stops it has started.
launch() {
    game="$1/$name/sf-game"
    cd /
    set +e
    timeout 25 xvfb-run -a -s "-screen 0 1920x1080x24" "$game" > "/tmp/launch-$2.log" 2>&1
    status=$?
    set -e
    cd "$HOME"
    cat "/tmp/launch-$2.log"
    [ "$status" -eq 124 ] || fail "the game stopped by itself (exit $status) on the $2 launch"
    echo "ok   the game stayed running on the $2 launch"
}

mkdir -p "$HOME/Games/first"
tar -xzf "$archive" -C "$HOME/Games/first"
game="$HOME/Games/first/$name/sf-game"
[ -x "$game" ] || fail "the extracted sf-game is not executable"

missing=$(ldd "$game" | grep "not found" || true)
[ -z "$missing" ] || fail "libraries are missing: $missing"
echo "ok   every linked library is present"
ldd "$game"

banner=$(cd / && "$game" --version)
echo "ok   version from the root folder: $banner"

launch "$HOME/Games/first" first
[ -f "$data/settings.toml" ] || fail "no settings.toml in $data after the first launch"
[ -f "$data/sf-game.log" ] || fail "no sf-game.log in $data after the first launch"
echo "ok   settings and the log are in $data"
cat "$data/sf-game.log"
grep -q "^master = 3$" "$data/settings.toml" && fail "the default master volume is already 3"

# A package update: change a setting, replace the package folder, launch again.
sed -i 's/^master = .*/master = 3/' "$data/settings.toml"
mkdir -p "$data/roms/sf"
echo keep > "$data/roms/sf/marker"
rm -rf "$HOME/Games/first"
mkdir -p "$HOME/Games/second"
tar -xzf "$archive" -C "$HOME/Games/second"
launch "$HOME/Games/second" second
grep -q "^master = 3$" "$data/settings.toml" || fail "the changed setting did not survive the package update"
[ -f "$data/roms/sf/marker" ] || fail "the player's ROM folder did not survive the package update"
echo "ok   settings and the ROM folder survived replacing the package"

# The window and the built-in looks, read from beside the executable while the
# working folder is somewhere else. Pacing checks need a real display and are
# reported, not required.
cd /
set +e
timeout 600 xvfb-run -a -s "-screen 0 1920x1080x24" "$HOME/Games/second/$name/sf-game" smoke > /tmp/smoke.log 2>&1
status=$?
set -e
cat /tmp/smoke.log
echo "note smoke exit status on the virtual display: $status"
for pack in art voice; do
    grep -q "^ok   $pack pack opens at $HOME/Games/second/$name/assets/sf1-port-$pack" /tmp/smoke.log \
        || fail "the $pack pack was not opened from beside the executable"
done
echo "ok   the art and voice packs open from beside the executable"
grep -q "^looks: $HOME/Games/second/$name/shaders$" /tmp/smoke.log || fail "the looks were not read from beside the executable"
echo "ok   the looks are read from beside the executable"
