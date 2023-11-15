{
  flake-parts,
  lib,
  defaultSystems
}:
rec {
  callFlake = import ./call-flake.nix;
  mkWorkspace = import ./create-workspace.nix {
    inherit defaultSystems flake-parts lib callFlake;
  };
}
