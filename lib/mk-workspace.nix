{ lib, callFlake, mkWorkspaceEnv }:
{
  src,
  inputs,
  cfgFile ? src + "/nixspace.toml",
  localFile ? src + "/.nixspace/local.json",
}: let
  cfg = builtins.fromTOML (builtins.readFile cfgFile);

  envNames = map (env: env.name) cfg.environments;


  # get the impure workspace root from the environment
  # used for loading editable packages
  findRoot = depth: path:
    if (depth > 100) then abort "could not find workspace root; directory depth 100 exceeded"
    else if (builtins.pathExists "${path}/nixspace.toml") then path
    else findRoot (depth + 1) "${path}/..";
  impureRoot = findRoot 1 (builtins.getEnv "PWD");
  local = if lib.inPureEvalMode then null
          else builtins.fromJSON (builtins.readFile "${impureRoot}/.nixspace/local.json");

  projectCfg = builtins.listToAttrs (builtins.map
    (project: { name = project.name; value = project; })
    cfg.projects
  );

  env = map (env: mkWorkspaceEnv {
    inherit inputs cfg projectCfg local impureRoot;
    lockFile = src + "/.nixspace/${env}.lock";
  }) envNames;
  ws = {
    inherit env cfg local;
    default = cfg.default_env;
  };
in ws // {
  flakeModule = { ... }: {
    flake = {
      lib.nixspace = ws;
    };
    perSystem = { pkgs, ... }: let
      nixspace = pkgs.callPackage ../. { };
    in {
      # TODO: add consumer/dependent flake resolution
      # apps = {
      #   ns-consumers = import ./ns-consumers.nix lib ws;
      #   ns-dependents = import ./ns-dependents.nix lib ws;
      # };
      devShells.default = pkgs.mkShell {
        packages = [
          nixspace
        ];
      };
    };
  };
}
