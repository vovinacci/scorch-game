{
  description = "Scorch: turn-based artillery game, a tribute to Scorched Earth (1991)";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      cargo = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      # Same scheme as build.rs: 0.1.0+gabc1234, or .dirty for uncommitted changes.
      rev = self.shortRev or self.dirtyShortRev or "unknown";
    in
    {
      packages = forAll (pkgs: rec {
        scorch = pkgs.rustPlatform.buildRustPackage {
          pname = "scorch";
          inherit (cargo.package) version;
          src = self;
          cargoLock.lockFile = ./Cargo.lock;
          # Nix updates the game itself.
          buildNoDefaultFeatures = true;

          nativeBuildInputs = [ pkgs.makeWrapper ];
          env.SCORCH_BUILD_VERSION = "${cargo.package.version}+g${pkgs.lib.removeSuffix "-dirty" rev}"
            + pkgs.lib.optionalString (pkgs.lib.hasSuffix "-dirty" rev) ".dirty";

          # The game loads these at run time (dlopen); NixOS has no standard library paths.
          postInstall = ''
            wrapProgram $out/bin/scorch --prefix LD_LIBRARY_PATH : ${pkgs.lib.makeLibraryPath (with pkgs; [
              libGL
              libx11
              libxi
              libxkbcommon
            ])}
            install -Dm644 packaging/scorch.desktop $out/share/applications/scorch.desktop
            install -Dm644 assets/icon/scorch-256.png $out/share/icons/hicolor/256x256/apps/scorch.png
          '';

          meta = {
            description = cargo.package.description;
            homepage = "https://github.com/vovinacci/scorch-game";
            license = pkgs.lib.licenses.mit;
            mainProgram = "scorch";
            platforms = systems;
          };
        };
        default = scorch;
      });

      # Development: the same shell as shell.nix.
      devShells = forAll (pkgs: { default = import ./shell.nix { inherit pkgs; }; });
    };
}
