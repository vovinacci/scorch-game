# NixOS: miniquad loads libGL, libX11, libXi and libxkbcommon at run time (dlopen), so they must be on
# LD_LIBRARY_PATH. Sound links ALSA (alsa-lib). These are all the game needs.
# Use: `nix-shell --run "cargo run"`
{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  # The commit message check (tools/verify_commit.sh). Python tools come from requirements.txt (`make tools`).
  packages = with pkgs; [ cocogitto ];
  buildInputs = with pkgs; [ alsa-lib ];
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
    alsa-lib
    libGL
    libx11
    libxi
    libxkbcommon
  ]);
}
