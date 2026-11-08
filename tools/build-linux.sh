#!/usr/bin/env bash
# Release build for Linux that runs on any distribution with glibc GLIBC_FLOOR or newer, whatever the build machine
# has: cargo-zigbuild links against that glibc version (tools pinned in requirements-build.txt). Then checks the binary
# and fails if it needs a newer glibc. The result goes to target/release/scorch, where tools/package.sh expects it.
#
#   tools/build-linux.sh   # needs python3, ALSA (libasound.so.2; or set ALSA_LIB to it) and objdump
set -euo pipefail

GLIBC_FLOOR=2.35 # also in README.md (system requirements)

arch=$(uname -m)
target="$arch-unknown-linux-gnu"
tmp=${RUNNER_TEMP:-$(mktemp -d)}

python3 -m venv "$tmp/zigbuild"
"$tmp/zigbuild/bin/pip" install --quiet --disable-pip-version-check --require-hashes -r requirements-build.txt
export PATH="$tmp/zigbuild/bin:$PATH"

# Only libasound in the linker path: the system folder would also offer the build machine's own glibc.
alsa=${ALSA_LIB:-$(find /usr/lib /usr/lib64 /lib -name 'libasound.so.2' -print -quit 2>/dev/null)}
[ -n "$alsa" ] || { echo "libasound.so.2 not found: install ALSA (libasound2-dev)" >&2; exit 1; }
mkdir -p "$tmp/link"
ln -sf "$alsa" "$tmp/link/libasound.so"
export RUSTFLAGS="${RUSTFLAGS:-} -L native=$tmp/link"

cargo zigbuild --release --locked --target "$target.$GLIBC_FLOOR"

out=${CARGO_TARGET_DIR:-target}
binary="$out/$target/release/scorch"
needed=$(objdump -T "$binary" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -uV | tail -1)
echo "scorch needs glibc $needed (floor $GLIBC_FLOOR)"
if [ "$(printf '%s\n%s\n' "$needed" "$GLIBC_FLOOR" | sort -V | tail -1)" != "$GLIBC_FLOOR" ]; then
    echo "scorch needs glibc $needed, newer than the promised $GLIBC_FLOOR" >&2
    exit 1
fi

mkdir -p "$out/release"
cp "$binary" "$out/release/scorch"
