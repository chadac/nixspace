{ lib, callFlake }:
{
  inputs,
  cfg,
  # cfgFile ? src + "/nixspace.toml",
  lockFile,
  local,
  # localFile ? src + "/.nixspace/local.json",
}: let
  # TODO: Editable Projects MUST have a flake.lock
  lock = builtins.fromJSON (builtins.readFile lockFile);

  findRoot = depth: path:
    if (depth > 100) then abort "could not find workspace root; directory depth 100 exceeded"
    else if (builtins.pathExists "${path}/nixspace.toml") then path
    else findRoot (depth + 1) "${path}/..";
  impureRoot = findRoot 1 (builtins.getEnv "PWD");
  local = builtins.fromJSON (builtins.readFile "${impureRoot}/.nixspace/local.json");
  projectCfg = builtins.listToAttrs (builtins.map
    (project: { name = project.name; value = project; })
    cfg.projects
  );
  projects = builtins.mapAttrs (name: inputSpec:
    if ((builtins.hasAttr name local.projects) && local.projects.${name}.editable)
    then builtins.fetchTree {
      type = "path";
      path = impureRoot + "/" + projectCfg.${name}.path;
    }
    else builtins.fetchTree inputSpec.locked
  ) lock.nodes;
  wsNodes =
    inputs
    // (
      builtins.mapAttrs (name: tree: let
        rootSrc = tree.outPath;
        projLock = rootSrc + "/flake.lock";
        lockFileStr =
          if (builtins.pathExists projLock)
          then builtins.readFile (rootSrc + "/flake.lock")
          else ''{"nodes": {"root": {}}, "root": "root", "version": 7}''
        ;
      in
        callFlake wsNodes lockFileStr tree ""
      ) projects
    )
  ;
  # wsModule = { ... }: {
  #   perSystem = { pkgs, system, ... }: let
  #     inherit (lib) concatMapAttrs nameValuePair mapAttrs';
  #     mapPackages = property: concatMapAttrs (package: flake:
  #       mapAttrs' (name: value: nameValuePair "${package}.${name}" value)
  #         flake.${property}.${system}
  #     ) projects;
  #   in builtins.listToAttrs (property:
  #     { name = property; value = mapPackages property; }
  #   ) [ "apps" "checks" "packages" "devShells" ];
  # };
in {
  projects = wsNodes;
}
