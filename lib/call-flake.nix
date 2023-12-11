# from: https://github.com/NixOS/nix/blob/c0c7c4b6cd1aefaa65fc11fcdc8df7e608960825/src/libexpr/flake/call-flake.nix
#
# MODIFICATIONS
#
# 2023-11-08(chad@cacrawford.org): modified to include overrides from a workspace
#

overrides: lockFileStr: rootSrc: rootSubdir:

let
  lockFile = builtins.fromJSON lockFileStr;

  allNodes =
    builtins.mapAttrs
      (key: node:
        let

          sourceInfo =
            if key == lockFile.root
            then rootSrc
            else if overrides ? key
            then overrides.${key}
            else fetchTree (node.info or {} // removeAttrs node.locked ["dir"]);

          subdir = if key == lockFile.root then rootSubdir else node.locked.dir or "";

          outPath = sourceInfo + ((if subdir == "" then "" else "/") + subdir);

          flake = import (outPath + "/flake.nix");

          inputs = builtins.mapAttrs
            (inputName: inputSpec: allNodes.${resolveInput inputSpec})
            (node.inputs or {});

          # Resolve a input spec into a node name. An input spec is
          # either a node name, or a 'follows' path from the root
          # node.
          resolveInput = inputSpec:
              if builtins.isList inputSpec
              then getInputByPath lockFile.root inputSpec
              else inputSpec;

          # Follow an input path (e.g. ["dwarffs" "nixpkgs"]) from the
          # root node, returning the final node.
          getInputByPath = nodeName: path:
            if path == []
            then nodeName
            else
              getInputByPath
                # Since this could be a 'follows' input, call resolveInput.
                # Prefer resolving any inputs in our workspace first
                # (if wsNodes ? ${nodeName}
                #  then
                #    (resolveInput wsNodes.${nodeName}.inputs.${builtins.head path})
                #  else
                (resolveInput lockFile.nodes.${nodeName}.inputs.${builtins.head path})
                (builtins.tail path);

          outputs = flake.outputs (inputs // { self = result; });

          result =
            outputs
            # We add the sourceInfo attribute for its metadata, as they are
            # relevant metadata for the flake. However, the outPath of the
            # sourceInfo does not necessarily match the outPath of the flake,
            # as the flake may be in a subdirectory of a source.
            # This is shadowed in the next //
            // sourceInfo
            // {
              # This shadows the sourceInfo.outPath
              inherit outPath;

              inherit inputs; inherit outputs; inherit sourceInfo; _type = "flake";
            };

        in
          if node.flake or true then
            assert builtins.isFunction flake.outputs;
            result
          else
            sourceInfo
      )
      lockFile.nodes;

in allNodes.${lockFile.root}
