#!/usr/bin/env bash
#
# Release build for Linux that runs on any distribution with glibc GLIBC_FLOOR or newer, whatever the build machine
# has: cargo-zigbuild links against that glibc version (tools pinned in requirements-build.txt). Then checks the binary
# and fails if it needs a newer glibc. The result goes to target/release/scorch, where tools/package.sh expects it.
#
#   tools/build-linux.sh   # needs python3, ALSA (libasound.so.2; or set ALSA_LIB to it) and objdump

set -euf -o pipefail
export LC_ALL="C.UTF-8"
PS4='+ $(date -u +%Y-%m-%dT%H:%M:%S%z) ${BASH_SOURCE[0]##*/}:${LINENO}: '

readonly GLIBC_FLOOR="2.35" # also in README.md (system requirements)

# Installs zig and cargo-zigbuild into a venv under $1 and puts them on PATH.
function install-zigbuild() {
  local tmp="$1"
  python3 -m venv "${tmp}/zigbuild"
  "${tmp}/zigbuild/bin/pip" install --quiet --disable-pip-version-check --require-hashes -r requirements-build.txt
  export PATH="${tmp}/zigbuild/bin:${PATH}"
}

# Puts only libasound in a folder under $1 for the linker: the system folder would also offer the build machine's own
# glibc.
function link-alsa() {
  local tmp="$1"
  local alsa="${ALSA_LIB:-}"
  if [[ -z "${alsa}" ]]; then
    alsa="$(find /usr/lib /usr/lib64 /lib -name 'libasound.so.2' -print -quit 2>/dev/null)"
  fi
  if [[ -z "${alsa}" ]]; then
    echo "libasound.so.2 not found: install ALSA (libasound2-dev) or set ALSA_LIB" >&2
    exit 1
  fi
  mkdir -p "${tmp}/link"
  ln -sf "${alsa}" "${tmp}/link/libasound.so"
  # The build machine's libasound may itself need a newer glibc (Ubuntu 24.04's needs 2.38). That's fine: players load
  # their own libasound, built for their glibc. So the linker doesn't check that library's needs; check-glibc below
  # still covers everything scorch itself uses.
  export RUSTFLAGS="${RUSTFLAGS:-} -L native=${tmp}/link -C link-arg=-Wl,--allow-shlib-undefined"
}

# Fails if the binary $1 needs a glibc newer than GLIBC_FLOOR.
function check-glibc() {
  local binary="$1"
  local needed
  needed="$(objdump -T "${binary}" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -uV | tail -1)"
  echo "scorch needs glibc ${needed} (floor ${GLIBC_FLOOR})"
  if [[ "$(printf '%s\n%s\n' "${needed}" "${GLIBC_FLOOR}" | sort -V | tail -1)" != "${GLIBC_FLOOR}" ]]; then
    echo "scorch needs glibc ${needed}, newer than the promised ${GLIBC_FLOOR}" >&2
    exit 1
  fi
}

function main() {
  local target tmp out
  target="$(uname -m)-unknown-linux-gnu"
  tmp="${RUNNER_TEMP:-$(mktemp -d)}"
  out="${CARGO_TARGET_DIR:-target}"

  install-zigbuild "${tmp}"
  link-alsa "${tmp}"
  cargo zigbuild --release --locked --target "${target}.${GLIBC_FLOOR}"
  check-glibc "${out}/${target}/release/scorch"

  mkdir -p "${out}/release"
  cp "${out}/${target}/release/scorch" "${out}/release/scorch"
}

main "$@"

# EOF
