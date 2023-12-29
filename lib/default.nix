{
  self,
  lib,
  defaultSystems,
  revInfo,
}:
rec {
  callFlake = import ./call-flake.nix;
  mkWorkspaceEnv = import ./mk-workspace-env.nix {
    inherit lib callFlake;
  };
  mkWorkspace = import ./mk-workspace.nix {
    inherit lib callFlake mkWorkspaceEnv;
  };
  inherit revInfo;
}
