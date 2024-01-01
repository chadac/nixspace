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
  flatten =
    if(flattenFlakes != null) then flattenFlakes
    else if(builtins.hasAttr "flatten-flakes" cfg) then cfg.flatten-flakes
    else true;

  canFlatten = projectName: project:
    !builtins.hasAttr "flatten" projectCfg.${projectName} || projectCfg.${projectName}.flatten;

  flattenProject = flakeSection: projectName: project:
    if (project ? flakeSection) then
      lib.concatMapAttrs (name: value: {
        "${projectName}/${name}" = value;
      }) project.${flakeSection}
    else {}
  ;

  flattenSystemProject = flakeSection: system: projectName: project:
    if (builtins.hasAttr flakeSection project) then
      lib.concatMapAttrs
        (name: value: { "${projectName}/${name}" = value; })
        project.${flakeSection}.${system}
    else {}
  ;

  listToAttrs = list: builtins.listToAttrs (builtins.map (name: { inherit name; value = {}; }) list);
  flakeSystem = listToAttrs [ "packages" "apps" "devShells" "legacyPackages" "checks" ];
  flakeGeneral = listToAttrs [ "overlays" "nixosModules" ];

  flattenModule = projectName: project: { lib, env, ... }: {
    flake = lib.mkIf flatten (
      lib.mapAttrs
        (flakeSection: _: flattenProject flakeSection projectName project)
        flakeGeneral
    );

    perSystem = lib.mkIf flatten ({ system, ... }:
      lib.mapAttrs
        (flakeSection: _: flattenSystemProject flakeSection system projectName project)
        flakeSystem
    );
  };

  ws = builtins.mapAttrs (name: projects: let
    mkNsDevShell = pkgs: pkgs.mkShell {
      packages = [ self.packages.${pkgs.system}.nixspace ];
    };
    flattenProjects = lib.filterAttrs canFlatten projects;
  in projects // {
    inherit name;
    inherit projects;

    flake = let
      forAllSystems = lib.genAttrs systems;
      forAllProjects = flakeSection: _:
        lib.concatMapAttrs (flattenProject flakeSection) flattenProjects;
      forAllProjectsSystems = flakeSection: _:
        forAllSystems (system:
          lib.concatMapAttrs (flattenSystemProject flakeSection system) flattenProjects
        );
      f =
        if !empty && flatten then
          (lib.mapAttrs forAllProjectsSystems flakeSystem) //
          (lib.mapAttrs forAllProjects flakeGeneral)
        else { devShells = forAllSystems (system: { }); };
      devShells = forAllSystems (system:
        let pkgs = import inputs.nixpkgs { inherit system; };
        in { default = pkgs.mkShell { packages = [ self.packages.${system}.nixspace ]; }; }
      );
    in f // {
      devShells = lib.mapAttrs (system: shells:
        shells // devShells.${system}
      ) f.devShells;
    };

    # for use in flake-parts
    flakeModule = { ... }: {
      _module.args.env = name;

      inherit systems;

      imports = lib.attrValues (lib.mapAttrs flattenModule flattenProjects);

      perSystem = { pkgs, system, ... }: {
        devShells.default = pkgs.mkShell {
          packages = [ self.packages.${system}.nixspace ];
        };
      };
    };
  }) envs;
in ws // { default = ws.${cfg.default_env}; }
