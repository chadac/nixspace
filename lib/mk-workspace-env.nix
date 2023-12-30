{ lib, callFlake }:
{
  inputs,
  cfg,
  projectCfg,
  lockFile,
  local ? null,
  impureRoot ? null,
}: let
  # TODO: Editable Projects MUST have a flake.lock
  lock = builtins.fromJSON (builtins.readFile lockFile);

  projects = builtins.mapAttrs (name: inputSpec:
    if (local != null && (builtins.hasAttr name local.projects) && local.projects.${name}.editable)
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
in wsNodes
