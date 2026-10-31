# NixOS: miniquad loads libGL, libX11, libXi and libxkbcommon at run time (dlopen), so they must be on
# LD_LIBRARY_PATH. These four are all the game needs (checked by running it with only them).
# Use: `nix-shell --run "cargo run"`
{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  # The commit message check (tools/verify_commit.sh). Python tools come from requirements.txt (`make tools`).
  packages = with pkgs; [ cocogitto ];
  LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath (with pkgs; [
    libGL
    libx11
    libxi
    libxkbcommon
  ]);
}
