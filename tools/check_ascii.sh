#!/usr/bin/env bash
# Fails if the given files contain non-ASCII characters. Code and config stay ASCII; on-screen text belongs in
# assets/i18n/*.ftl. A line that really needs other characters (such as Ukrainian test data) ends with "ascii-ok".
status=0
for file in "$@"; do
    if grep -nP '[^\x00-\x7F]' "$file" | grep -v 'ascii-ok'; then
        echo "^ non-ASCII in $file" >&2
        status=1
    fi
done
exit $status
