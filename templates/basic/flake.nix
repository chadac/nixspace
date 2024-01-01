{
  description = "A sample workspace with nixspace";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    systems.url = "github:nix-systems/default";
    nixspace.url = "path:/home/chadac/code/github.com/chadac/nixspace";
  };

  outputs = { systems, nixspace, ... }@inputs: let
    ws = nixspace.lib.mkWorkspace {
      src = ./.;
      systems = import systems;
      inherit inputs;
    };
  in ws.default.flake;
}
