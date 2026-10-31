# NixOS: macroquad loads GL/X11/Wayland libraries at runtime via dlopen,
# so they must be on LD_LIBRARY_PATH. Use: `nix-shell --run "cargo run"`
{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  # Git hooks (prek.toml) and the Markdown linter/formatter (.rumdl.toml).
  packages = with pkgs; [ prek rumdl ];
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
    libGL
    libx11
    libxi
    libxcursor
    libxkbcommon
    wayland
    alsa-lib
  ]);
}
