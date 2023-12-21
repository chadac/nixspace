{
  lib,
  defaultSystems,
  revInfo,
}:
rec {
  callFlake = import ./call-flake.nix;
  mkWorkspace = import ./create-workspace.nix {
    inherit lib callFlake;
  };
  inherit revInfo;
}
