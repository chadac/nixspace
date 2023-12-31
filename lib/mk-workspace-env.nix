{ lib, callFlake }:
{
  inputs,
  cfg,
  projectCfg,
  lockFile,
  local ? null,
  impureRoot ? null,
}: let
  # wrapper for fetchTree so that I have all the necessary info in one place
  fetchFlake = flakeRef: let
    tree = builtins.fetchTree (builtins.removeAttrs flakeRef ["dir"]);
  in
    if flakeRef ? "dir" then tree // { rootDirectory = flakeRef.dir; }
    else tree // { rootDirectory = ""; } ;

  # TODO: Editable Projects MUST have a flake.lock
  lock = builtins.fromJSON (builtins.readFile lockFile);
  projectNames = lib.attrNames projectCfg;
  lockNodes = lib.filterAttrs (name: node: name != "root") lock.nodes;

  projects = builtins.mapAttrs (name: inputSpec:
    if (local != null && (builtins.hasAttr name local.projects) && local.projects.${name}.editable)
    then fetchFlake {
      type = "path";
      path = impureRoot + "/" + projectCfg.${name}.path;
    }
    else fetchFlake inputSpec.locked
  ) lockNodes;

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
        callFlake wsNodes lockFileStr tree tree.rootDirectory
      ) projects
    )
  ;
in lib.filterAttrs (name: node: builtins.elem name projectNames) wsNodes
