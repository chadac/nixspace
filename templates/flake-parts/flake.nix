{
  description = "A sample workspace flake using flake-parts with nixspace";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default";
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixspace.url = "path:/home/chadac/code/github.com/chadac/nixspace";
  };

  outputs = { flake-parts, systems, nixspace, ... }@inputs: let
    ws = nixspace.lib.mkWorkspace {
      src = ./.;
      systems = import systems;
      inherit inputs;
    };
  in flake-parts.lib.mkFlake { inherit inputs; } ({ ... }: {
    systems = import systems;
    imports = [ ws.default.flakeModule ];
  });
}
