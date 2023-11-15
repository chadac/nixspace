# nix-ws

Manage projects composed of multiple Nix packages inside a single
workspace. Useful for building manyrepo apps, testing entire
applications end-to-end to anticipate needs or side-effects from other
components, and for reproducible joint deployments.

`nix-ws` is like `npm` workspaces but for any Nix expression. You'll
end up with a workspace that look like:

    .
    ├── flake.nix              The workspace flake.nix
    ├── flake.lock             Flake lockfile
    ├── workspace.toml         Workspace configuration
    ├── workspace.lock         Tracks the version of all projects used.
    ├── .workspace-local.lock  Used for tracking differences between local and remote workspaces.
    ├── project-a              One of many Nix projects
    |   ├── flake.nix
    ├── project-b
    |   ├── flake.nix
    ├── subfolder
    |   ├── project-c
    |   |   ├── flake.nix

It has some additional capabilities, such as the ability to compose
many [numtide/devshell](https://github.com/numtide/devshell)'s
together to build a joint dev shell for a project.

## Why?

Building complex multi-project applications is a pain. Since each
project tends to be deployed inside its own CI pipeline and those
changes can propagate slowly, it's hard to track the exact version of
each project that is currently deployed at a single point in
time. This creates several issues:

1. Reproducing issues is a pain since it's hard to model interactions
   between services locally;
1. It's hard to anticipate side effects for new changes to libraries or
   services with downstream dependencies; and
1. There's additional management cruft such as unnecessary versioning
   libraries/services, coordinating deployments of multiple
   repositories at once, specialized logic about holding back specific
   services, etc.

## How it works

`nix-ws` creates reproducible workspaces that allow you to lock a
multi-project application down to the commit level per
project. Developers can clone and edit sub-projects, make changes, and
test how those changes would impact other packages in the workspace.

Your workspace configuration is tracked within three files:

1. `workspace.toml`: Your workspace configuration. Specifies which Git
   projects you are working on and what path they would be cloned to
   locally within the workspace.
2. `workspace.lock`: A git-committed file that locks down each project
   similar to a `flake.lock`. Workspaces layer on some additional
   logic to set up the proper `follows` links between projects, so
   that only one instance of each flake exists in the workspace.
3. `.workspace/local.lock`: An untracked file which allows the
   developer to diverge from the global `workspace.lock`. This would
   include pulling down later versions of all packages, cloning local
   copies and using those, etc.

## Installation

    nix shell github:chadac/nix-ws

## Usage

### Initializing

Start a new workspace in an empty folder with:

    nix flake init github:chadac/nix-ws

You may also run

    nix-ws init

### Adding

Add new packages with:

    nix-ws add github:my/project

`nix-ws` scans the project for any co-dependencies and will
automatically update the `workspace.toml` to properly follow any
dependencies, so if `project-b` depends on `project-a`, `nix-ws` will
properly set up `project-b.inputs.project-a.follows = "project-b"`.

### Sharing

Workspaces are git repositories, so if you commit and push the
workspace, then others can clone it with:

    nix-ws clone github:my/workspace

#### Initializing projects

By default, `nix-ws` clones all projects as stubs -- which means that
while they are built and tested, they aren't directly editable until
you explicitly **use** them.

You can initialize a project in a workspace with

    nix-ws use project-a

Syntax follows the expected folder path of the project:

    nix-ws use subfolder/project-c

If you would like to clone all packages in a workspace in editable
mode, run

    nix-ws clone github:my/workspace --use-all

#### Updating projects

To upgrade your `workspace.lock`, run

    nix-ws update

This will update all projects to use the latest available commit.

#### Syncing projects

To update all projects in a workspace to use the current versions of
each project that a workspace tracks, run

    nix-ws sync

If you would like to implicitly update the workspace to follow the
upstream, you may also run:

    nix-ws sync --remote

#### Combined devshell

If you want a workspace that combines many different devshells, simply
add to your outputs:

    devshells.default = nix-ws.lib.mergedevshells projects { };

Note that this is built for
[github:numtide/devshell](https://github.com/numtide/devshell).

### CI/CD

`nix-ws` projects can be a convenient means of deploying manyrepo
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


## Details

`nix-ws` is a small wrapper around flakes with some additional
utilities to make it easy to work on multiple projects in an
application at once.

