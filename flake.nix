{
  description = "Create workspaces to manage multiple packages with Nix.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    nixpkgs-lib.url = "github:NixOS/nixpkgs/nixpkgs-unstable?dir=lib";
    systems.url = "github:nix-systems/default";
  };

  outputs = inputs@{ self, nixpkgs, nixpkgs-lib, systems, ... }:
    let
      lib = import nixpkgs-lib;
      defaultSystems = import systems;
      eachSystem = lib.genAttrs defaultSystems;
    in {
      lib = import ./lib {
        inherit self;
        inherit lib;
        inherit defaultSystems;
        revInfo =
          if lib?rev
          then " (nixpkgs-lib.rev: ${lib.rev})"
          else "";
      };
    } // {
      packages = eachSystem (system: let
        pkgs = import nixpkgs { inherit system; };
        nixspace = pkgs.callPackage ./. { };
      in {
        inherit nixspace;
        default = nixspace;
      });
      devShells = eachSystem (system: let
        pkgs = import nixpkgs { inherit system; };
      in {
        default = pkgs.mkShell {
          packages = with pkgs; [
            cargo
            rustc
            cargo-watch
            clippy
          ];
        };
      });
    };
}
