# NixOS: miniquad loads libGL, libX11, libXi and libxkbcommon at run time (dlopen), so they must be on
# LD_LIBRARY_PATH. Sound links ALSA (alsa-lib). These are all the game needs.
# Use: `nix-shell --run "cargo run"`
{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  # The commit message check (tools/verify_commit.sh), and Python 3.15 (mise.toml) for the tools in requirements.txt
  # (`make tools`).
  packages = with pkgs; [ cocogitto python315 ];
  buildInputs = with pkgs; [ alsa-lib ];
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
    alsa-lib
    libGL
    libx11
    libxi
    libxkbcommon
  ]);
}
