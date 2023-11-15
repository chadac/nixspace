{ defaultSystems, flake-parts, lib, callFlake }:
{
  src,
  inputs,
  cfgFile ? src + "/workspace.toml",
  lockFile ? src + "/workspace.lock",
  localLockFile ? src + "/.workspace/local.lock",
  systems ? defaultSystems,
}: module: let
  cfg = builtins.fromTOML (builtins.readFile cfgFile);
  lock = builtins.fromJSON (builtins.readFile lockFile);
  local = builtins.fromJSON (builtins.readFile localLockFile);
  projects = builtins.mapAttrs (name: inputSpec:
    if local.${name}
    then builtins.getTree { path = "path:./" + src + cfg.${name}.path; }
    else builtins.getTree inputSpec
  ) lock.nodes;
  wsNodes =
    inputs
    // (
      builtins.mapAttrs (tree: let
        lockFileStr = builtins.readFile (src + "/flake.lock");
        rootSrc = tree.outPath;
      in
        callFlake wsNodes lockFileStr rootSrc ""
      ) projects
    );
  wsModule = { ... }: {
    perSystem = { pkgs, system, ... }: let
      inherit (lib) concatMapAttrs nameValuePair mapAttrs';
      mapPackages = property: concatMapAttrs (package: flake:
        mapAttrs' (name: value: nameValuePair "${package}.${name}" value)
          flake.${property}.${system}
      ) projects;
    in builtins.listToAttrs (property:
      { name = property; value = mapPackages property; }
    ) [ "apps" "checks" "packages" "devShells" ];
  };
in flake-parts.lib.mkFlake { inherit inputs; } {
  _module.args.projects = projects;

  imports = [ wsModule module ];
  inherit systems;
}
