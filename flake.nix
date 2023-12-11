{
  description = "Create workspaces to manage multiple packages with Nix.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    nixpkgs-lib.url = "github:NixOS/nixpkgs/nixpkgs-unstable?dir=lib";
    systems.url = "github:nix-systems/default";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
    };
  };

  outputs = inputs@{ flake-parts, nixpkgs-lib, systems, ... }:
    let
      defaultSystems = import systems;
    in flake-parts.lib.mkFlake { inherit inputs; } {
      flake = {
        lib = import ./lib {
          inherit (nixpkgs-lib) lib;
          inherit defaultSystems;
          revInfo =
            if nixpkgs-lib?rev
            then " (nixpkgs-lib.rev: ${nixpkgs-lib.rev})"
            else "";
        };
      };

      systems = defaultSystems;

      perSystem = { pkgs, ... }: let
        nixspace = pkgs.callPackage ./. { };
      in {
        packages.default = nixspace;
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            rustup
            cargo
            cargo-watch
            clippy
          ];
        };
      };
    };
}
