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
* *Space-efficient*: Unlike `git` submodules, `nixspace` doesn't
  require cloning down every project in a workspace. Developers can
  `add` the projects they'd like to work on and test them in
  integration in the workspace without pulling down all
  packages. `nixspace` can scale to track the state of thousands of
  applications at once.
* *Composable dev environments*: `nixspace`s allow developers to
  seamlessly compose the development environments of multiple projects
  together.

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

A standard nixspace looks like:

    .
    ├── flake.nix              The nixspace flake.nix
    ├── flake.lock             Flake lockfile
    ├── nixspace.yml           Nixspace configuration
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

## Installation

    nix shell github:chadac/nixspace

## Usage

### Initializing

Start a new workspace in an empty folder with:

    nix flake init github:chadac/nixspace

You may also run

    nixspace init

### Adding

Add new packages with:

    nixspace add github:my/project

`nixspace` scans the project for any co-dependencies and will
automatically update the `workspace.toml` to properly follow any
dependencies, so if `project-b` depends on `project-a`, `nixspace` will
properly set up `project-b.inputs.project-a.follows = "project-b"`.

### Sharing

Workspaces are git repositories, so if you commit and push the
workspace, then others can clone it with:

    nixspace clone github:my/workspace

#### Initializing projects

By default, `nixspace` clones all projects as stubs -- which means that
while they are built and tested, they aren't directly editable until
you explicitly **use** them.

You can initialize a project in a workspace with

    nixspace use project-a

Syntax follows the expected folder path of the project:

    nixspace use subfolder/project-c

If you would like to clone all packages in a workspace in editable
mode, run

    nixspace clone github:my/workspace --use-all

#### Updating projects

To upgrade your `workspace.lock`, run

    nixspace update

This will update all projects to use the latest available commit.

#### Syncing projects

To update all projects in a workspace to use the current versions of
each project that a workspace tracks, run

    nixspace sync

If you would like to implicitly update the workspace to follow the
upstream, you may also run:

    nixspace sync --remote

#### Combined devshell

If you want a workspace that combines many different devshells, simply
add to your outputs:

    devshells.default = nixspace.lib.mergedevshells projects { };

Note that this is built for
[github:numtide/devshell](https://github.com/numtide/devshell).

### CI/CD

`nixspace` projects can be a convenient means of deploying manyrepo
applications in a joint, lockable format that is easy to audit.

For example, with GitLab CI:

    image: nixos/nix

    build:
        steps:
            - nix build .#all.default

    integ:
        steps:
            - nix run .#project-c.integ-tests

    release:
        steps:
            - nix run .#project-c.deploy

## TODO

* *Combined merge requests*: It'd be nice if we could automate
  generating merge requests across multiple repositories and linking
  them into a single deployment.
