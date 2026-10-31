#!/usr/bin/env bash
# commit-msg hook: the message must be a Conventional Commit (https://www.conventionalcommits.org), checked by
# cocogitto. On NixOS cog comes from shell.nix; elsewhere install it (cargo install cocogitto).
set -euo pipefail
file=$1
if command -v cog >/dev/null; then
    exec cog verify --ignore-merge-commits --ignore-fixup-commits --file "$file"
elif [ -e /etc/NIXOS ]; then
    exec nix-shell --run "cog verify --ignore-merge-commits --ignore-fixup-commits --file '$file'"
else
    echo "cog (cocogitto) is not installed: cargo install cocogitto" >&2
    exit 1
fi
