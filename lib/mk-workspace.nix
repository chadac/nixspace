{ self, lib, callFlake, mkWorkspaceEnv }:
{
  src,
  inputs,
  systems,
  cfgFile ? src + "/nixspace.toml",
  localFile ? src + "/.nixspace/local.json",
  flattenFlakes ? null,
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

  projectCfg =
    if (cfg ? "projects") then
      builtins.listToAttrs (builtins.map
        (project: { name = project.name; value = project; })
        cfg.projects)
    else {}
  ;

  envs = builtins.listToAttrs (map (env: {
    name = env;
    value = mkWorkspaceEnv {
      inherit inputs cfg projectCfg local impureRoot;
      lockFile = src + "/.nixspace/${env}.lock";
    };
  }) envNames);

  empty = builtins.length (builtins.attrNames projectCfg) == 0;

  canFlakeIncludeProject = flakeAttr: projectName: flake:
    builtins.hasAttr flakeAttr flake && !(projectCfg.${projectName}.exclude or false);

  flakeAttrs = [ "overlays" "nixosModules" ];
  flakeSystemAttrs = [ "packages" "apps" "devShells" "legacyPackages" "checks" ];

  renameFlakeAttrs = projectName: attrs:
    lib.concatMapAttrs (name: value: {
      "${projectName}/${name}" = value;
    }) attrs;

  mkFlakeModule = flakeAttr: projectName: flake: { ... }: {
    flake.${flakeAttr} = lib.mkIf
      (canFlakeIncludeProject flakeAttr projectName flake)
      (renameFlakeAttrs projectName flake.${flakeAttr});
  };

  mkPerSystemModule = flakeAttr: projectName: flake: { ... }: {
    perSystem = { system, ... }: {
      ${flakeAttr} = lib.mkIf
        (canFlakeIncludeProject flakeAttr projectName flake)
        (renameFlakeAttrs projectName (flake.${flakeAttr}.${system} or {}));
    };
  };

  ws = builtins.mapAttrs (name: projects: let
    flakeModule = { ... }: let
      imports = builtins.concatMap (flakeAttr:
        builtins.attrValues (
          builtins.mapAttrs (mkFlakeModule flakeAttr) projects
        )
      ) flakeAttrs;
    in {
      imports = imports;
    };
    perSystemModule = { ... }: {
      imports = builtins.concatMap (flakeAttr:
        builtins.attrValues (
          builtins.mapAttrs (mkPerSystemModule flakeAttr) projects
        )
      ) flakeSystemAttrs;
    };
  in projects // {
    inherit name;
    inherit projects;

    # for use in flake-parts
    flakeModule = { ... }: {
      _module.args.env = name;

      inherit systems;
      imports = [
        flakeModule
        perSystemModule
      ];

      perSystem = { pkgs, system, ... }: {
        devShells.default = pkgs.mkShell {
          packages = [ self.packages.${system}.nixspace ];
        };
      };
    };
  }) envs;
in ws // { default = ws.${cfg.default_env}; }
