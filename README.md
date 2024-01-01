# nixspace

`nixspace` is a Nix Flake-based workspace manager for manyrepo
projects. Similar to `npm` workspaces, `nixspace` enables developers
to:

* Seamlessly work on multiple projects;
* Test projects in integration with ease; and
* Deploy the application in a unified fashion, ensuring that you have
  locked the entire state of your ecosystem down to the commit of
  every project used.

Unlike `npm` workspaces, `nixspace`s are Flake-based and are thus more
flexible:

* Supports both monorepo and manyrepo development styles;
* Is unopinionated about programming language. It can be used to host
  projects under any number of languages; and
* Can be used to perform joint deployments of applications, ensuring
  that the workspace corresponds exactly to the external state.

## Features

* *Lockfiles*: Track the exact commit of every project in the
  workspace with a lockfile, formatted identically to the `flake.lock`.
* *Environments*: Workspaces can be used to lock the state of multiple
  deployment environments for environments that track rolling upgrades.
* *Customized upgrades*: Specify on a project level how upgrades are
  consumed (follow branches, latest tags, semver, etc).
* *Clean*: Unlike `git` submodules, `nixspace` doesn't require cloning
  down every project in a workspace. Developers can `add` the projects
  they'd like to work on and test them in integration in the workspace
  without pulling down all packages. `nixspace` can scale to track the
  state of thousands of applications at once.

## How it works

`nixspace` is made of two components:

1. A Nix library for managing multi-Flake projects. This library helps
   manage automatically detecting and substituting `passthru`'s in
   inputs and makes it easy to test changes on the consumers of a
   package. (i.e., does application Y's unit tests still pass when I
   update library X?)
2. A CLI utility for managing the nixspace. This includes convenience
   utilities for maintaining and updating workspace lockfiles, as well as
   making it simple to edit any projects in the `nixspace`.

A standard workspace looks like:

    .
    ├── flake.nix              The nixspace flake.nix
    ├── flake.lock             Flake lockfile
    ├── nixspace.toml          Nixspace configuration
    ├── .nixspace
    |   ├── prod.lock          Nixspace package lockfile (formatted like a standard Flake lockfile)
    |   ├── dev.lock           Nixspaces can manage multiple environments
    |   ├── nixspace.local     Used for tracking differences between local and remote workspaces.
    ├── project-a              One of many Nix projects
    |   ├── flake.nix
    ├── project-b
    |   ├── flake.nix
    ├── subfolder
    |   ├── project-c
    |   |   ├── flake.nix

## Getting Started

To start using the CLI, run:

    nix shell github:chadac/nixspace

Create a new workspace with:

    ns init --name <your-workspace-name>
    cd <your-workspace-name>

You may also use `ns init --type flake-parts --name
<your-workspace-name>` to initialize a workspace with a `flake.nix`
that [flake-parts](https://flake.parts/) compatible.

### Registering and editing projects

To add a new project to your workspace, run

    ns register github:my/project --name my-project --path ./my-project

This will register your project as part of the workspace -- now, any
other projects that have an input named `my-project` in their
`flake.nix` will use the workspace copy instead. Ensure that the name
passed to `--name` is unique and distinguishible, as it is used to
determine what input to replace in every project's `flake.nix`.

By default, projects added to a workspace are not *editable*. This
means that they are initially not cloned into your workspace and are
not locally editable.

To edit any project in the workspace, run

    ns edit my-project

This will clone the project into the path specified in the `register`
command, and will link the project to the workspace so that it is
fully editable.

### Testing changes

Suppose `my-project` is dependent on `shared-project`, and both are
registered to the workspace and marked as editable. To test a local
change in `shared-project` on `my-project`, you only need to navigate
into `my-project` and run:

    ns build .#my-package-or-app

`ns` is a small alias for `nix` that replaces the project's
flake-specific lock information with the workspace lock. Since `ns`
runs in impure mode, editable projects are linked in their present,
local state. Therefore, no other steps are needed -- you can
immediately see the effects of one flake on another without any need
for running `nix flake update` or pushing commits to a repository.

## TODO

* *Test changes on all consumers*: It'd be nice to have something like
  `ns build .# --all-consumers` that would run a build instead on
  every package that depends on a flake. Sort of like a reverse
  closure. Gotta write some Nix hacks to do this.
* *Composable dev environments*: `nixspace`s allow developers to
  seamlessly compose the development environments of multiple projects
  together.
* *Combined merge requests*: It'd be nice if we could automate
  generating merge requests across multiple repositories and linking
  them into a single deployment.
