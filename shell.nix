# NixOS: macroquad loads GL/X11/Wayland libraries at runtime via dlopen,
# so they must be on LD_LIBRARY_PATH. Use: `nix-shell --run "cargo run"`
{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  # The commit message check (tools/verify_commit.sh). Python tools come from requirements.txt (`make tools`).
  packages = with pkgs; [ cocogitto ];
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
