#!/usr/bin/env bash
# Packages release builds into dist/ for the release workflow.
#
#   tools/package.sh linux   <arch> <version>   # target/release/scorch -> tar.gz + AppImage
#   tools/package.sh windows <arch> <version>   # target/release/scorch.exe -> zip
#   tools/package.sh macos   <version>          # universal Scorch.app from both Apple targets -> zip
#
# <arch> is x86_64 or aarch64; <version> is the tag, e.g. v0.1.0. Icons come from the PNGs in assets/icon/ (rendered
# from scorch.svg). Needs appimagetool for the AppImage (APPIMAGETOOL, default "appimagetool"), 7z on Windows, and
# lipo/iconutil/ditto on macOS.
set -euo pipefail

os=$1
mkdir -p dist
docs=(README.md LICENSE docs/GUIDE.md docs/GUIDE.uk.md)

stage() { # $1 = folder name; copies the docs into it
    rm -rf "dist/$1"
    mkdir -p "dist/$1"
    cp "${docs[@]}" "dist/$1/"
}

case $os in
linux)
    arch=$2 version=$3
    name="scorch-$version-linux-$arch"
    stage "$name"
    cp target/release/scorch "dist/$name/"
    tar -C dist -czf "dist/$name.tar.gz" "$name"

    app=dist/Scorch.AppDir
    rm -rf "$app"
    mkdir -p "$app/usr/bin"
    cp target/release/scorch "$app/usr/bin/"
    cp packaging/scorch.desktop "$app/"
    cp assets/icon/scorch-256.png "$app/scorch.png"
    ln -s usr/bin/scorch "$app/AppRun"
    ARCH=$arch APPIMAGE_EXTRACT_AND_RUN=1 "${APPIMAGETOOL:-appimagetool}" --no-appstream "$app" \
        "dist/Scorch-$version-$arch.AppImage"
    ;;
windows)
    arch=$2 version=$3
    name="scorch-$version-windows-$arch"
    stage "$name"
    cp target/release/scorch.exe "dist/$name/"
    (cd dist && 7z a -tzip -bso0 "$name.zip" "$name")
    ;;
macos)
    version=$2
    name="scorch-$version-macos"
    stage "$name"
    app="dist/$name/Scorch.app"
    mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
    lipo -create -output "$app/Contents/MacOS/scorch" \
        target/aarch64-apple-darwin/release/scorch target/x86_64-apple-darwin/release/scorch
    sed "s/@VERSION@/${version#v}/" packaging/Info.plist > "$app/Contents/Info.plist"
    iconset=dist/Scorch.iconset
    mkdir -p "$iconset"
    for size in 16 32 128 256 512; do
        cp "assets/icon/scorch-$size.png" "$iconset/icon_${size}x${size}.png"
        cp "assets/icon/scorch-$((size * 2)).png" "$iconset/icon_${size}x${size}@2x.png"
    done
    iconutil -c icns "$iconset" -o "$app/Contents/Resources/Scorch.icns"
    # Ad-hoc signature: Apple Silicon refuses unsigned apps outright ("damaged"). Gatekeeper still rejects ad-hoc apps;
    # players clear the quarantine mark with xattr (README). Opening normally would need a Developer ID + notarization.
    codesign --force --deep --sign - "$app"
    codesign --verify --deep --strict --verbose=2 "$app"
    (cd dist && ditto -c -k --keepParent "$name" "$name.zip")
    ;;
*)
    echo "unknown platform: $os" >&2
    exit 1
    ;;
esac
