#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod workspace;
mod config;
mod lockfile;
mod flake;
mod cli;

use crate::config::Config;
use crate::cli::{CliCommand, Git, Nix};
use crate::workspace::{ProjectRef, Workspace};

use anyhow::{anyhow, bail, Error, Result};
use clap::{Args, Parser, Subcommand};
use std::path::Path;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    // WORKSPACE COMMANDS
    /// create an empty workspace
    ///
    /// equivalent to `nix flake init github:chadac/nix-ws`
    Init(Init),
    /// clone a workspace
    Clone(Clone),
    /// shows the layout of the current workspace.
    Show(Show),

    // SUBCOMMANDS
    /// workspace configuration management
    #[command(subcommand)]
    Config(ConfigSubcommand),

    /// manage workspace environments
    #[command(subcommand)]
    Env(EnvSubcommand),

    // PROJECT COMMANDS
    /// import a project to the workspace.
    Attach(Attach),
    /// erase a project from the workspace.
    Detach(Detach),

    /// link a project locally into the workspace;
    /// i.e., clone it and make it editable.
    Link(Link),
    /// unlink the project from the workspace
    Unlink(Unlink),

    // GIT MANAGEMENT
    /// pull the workspace config + lockfile from the upstream remote
    Sync(Sync),
    /// publish the workspace config + lockfile to the upstream remote
    Publish(Publish),

    // LOCKFILE MANAGEMENT
    /// update the workspace lockfile
    Update(Update),

    // NIX ALIASES
    Build(NixArgs),
    Run(NixArgs),
    Flake(NixArgs),
}

trait Command {
    fn run(&self) -> Result<()>;
}

#[derive(Args, Debug)]
struct Init {
    /// name of the workspace
    #[arg(short, long)]
    name: String,
}

impl Command for Init {
    fn run(&self) -> Result<()> {
        let dir = Path::new(&self.name);
        if dir.exists() {
            bail!("error: path already exists");
        }
        let cmd = ["flake", "init", "-t", "github:chadac/nix-ws"];
        Nix::exec(&cmd, &dir)?;
        Git::init(&dir)?;
        let ws = Workspace::at(&dir)?;
        ws.commit("initial commit")?;
        Ok(())
    }
}

#[derive(Args, Debug)]
struct Clone {
    /// flake reference for the workspace
    flake_ref: String,
    /// name of the directory to clone the workspace into
    directory: Option<String>,
    /// if present, clone all projects locally
    #[arg(long)]
    clone_all: bool,
}

impl Command for Clone {
    fn run(&self) -> Result<()> {
        let input_spec = flake::InputSpec::parse(&self.flake_ref)?;
        let dest: String = match &self.directory {
            Some(dirname) => dirname.to_string(),
            _ =>
                input_spec.owner.expect("could not infer project name from input spec; specify --directory for the destination dir."),
        };
        Nix::clone(&self.flake_ref, &dest, ".")?;
        Ok(())
    }
}

#[derive(Args, Debug)]
struct Show {
}

impl Command for Show {
    fn run(&self) -> Result<()> {
        todo!()
    }
}

#[derive(Debug, Subcommand)]
enum ConfigSubcommand {
    /// get a configuration value
    Get(ConfigGet),
    /// set a configuration value
    Set(ConfigSet),
}

impl Command for ConfigSubcommand {
    fn run(&self) -> Result<()> {
        todo!()
    }
}

#[derive(Args, Debug)]
struct ConfigGet {
    // configuration name
    name: String,
}

#[derive(Args, Debug)]
struct ConfigSet {
    // configuration name
    name: String,
    // configuration value
    value: String,
}

#[derive(Debug, Subcommand)]
enum EnvSubcommand {
    /// get a configuration value
    Get(EnvGet),
    /// set a configuration value
    Set(EnvSet),
}

impl Command for EnvSubcommand {
    fn run(&self) -> Result<()> {
        todo!()
    }
}

#[derive(Args, Debug)]
struct EnvGet {
    // environment name
    env: String,
    // configuration name
    name: String,
}

#[derive(Args, Debug)]
struct EnvSet {
    // environment name
    env: String,
    // configuration name
    name: String,
    // configuration value
    value: String,
}


impl Command for EnvGet {
    fn run(&self) -> Result<()> {
        todo!()
    }
}

#[derive(Args, Debug)]
struct Attach {
    /// path or reference to the project.
    path_or_ref: String,
    /// name of the directory that the project will be cloned into when added.
    /// default is the name of the project at the root of the workspace.
    #[arg(short, long)]
    directory: Option<String>,
    /// if present, clones the project locally
    #[arg(long)]
    edit: bool,
}

impl Command for Attach {
    fn run(&self) -> Result<()> {
        let mut ns = Workspace::discover()?;
        let project = match ns.project(&self.path_or_ref)? {
            Some(p) => p,
            None => {
                if let Some(dir) = &self.directory {
                    ns.add_project(&self.path_or_ref, &dir)?
                } else {
                    Err(Error::msg("--directory must be specified when adding a new project."))?
                }
            }
        };
        if self.edit {
            Nix::clone(&project.flake_ref.url, &project.config.path, ".")?;
            let name = project.config.name.clone();
            ns.mark_editable(&name);
        }
        ns.save()?;
        Ok(())
    }
}

#[derive(Args, Debug)]
struct Detach {
    /// path or reference to the project
    path_or_ref: String,
    #[arg(long)]
    /// if present, delete the directory from the workspace
    delete: bool,
}

impl Command for Detach {
    fn run(&self) -> Result<()> {
        todo!()
    }
}

#[derive(Args, Debug)]
struct Link {
    /// path or reference to the project
    path_or_ref: String,
}

impl Command for Link {
    fn run(&self) -> Result<()> {
        todo!()
    }
}

#[derive(Args, Debug)]
struct Unlink {
    /// path or flake reference to the project.
    path_or_ref: String,
    /// if present, deletes the project locally
    #[arg(long)]
    rm: bool
}

impl Command for Unlink {
    fn run(&self) -> Result<()> {
        let mut ws = Workspace::discover()?;
        let project = ws.remove_project(&self.path_or_ref)?;
        if self.rm {
            // TODO: Additional prompt for input
            std::fs::remove_dir_all(&project.config.path)?;
        }
        Ok(())
    }
}

#[derive(Args, Debug)]
struct Sync {
    /// will sync all local repositories with upstream
    #[arg(long)]
    local: bool
}

impl Command for Sync {
    fn run(&self) -> Result<()> {
        let mut ws = Workspace::discover()?;
        ws.sync()?;
        for project in ws.projects() {
            if self.local && project.editable {
                project.sync()?
            }
        }
        Ok(())
    }
}

#[derive(Args, Debug)]
struct Publish {
    /// commit message to include in publish
    #[arg(short, long)]
    message: String,
    #[arg(short, long)]
    force: bool,
}

impl Command for Publish {
    fn run(&self) -> Result<()> {
        let ws = Workspace::discover()?;
        ws.commit(&self.message)?;
        ws.publish(self.force)?;
        Ok(())
    }
}

#[derive(Args, Debug)]
struct Update {
    /// path or flake reference to the project.
    path_or_ref: Vec<String>,
    /// environment to update
    env: Option<String>,
    /// if present, publishes the new lockfile to the Git repository
    #[arg(long)]
    publish: bool,
}

impl Command for Update {
    fn run(&self) -> Result<()> {
        let mut ws = Workspace::discover()?;
        ws.update_lock(&self.env, &self.path_or_ref)?;
        ws.save()?;
        if self.publish {
            if ws.tracks_latest()? {
                ws.commit("chore: update workspace")?;
                ws.publish(false)?;
            }
            else {
                bail!("cannot commit; upstream is ahead of local git repository.")
            }
        }
        Ok(())
    }
}

#[derive(Args, Debug)]
struct NixArgs {
    #[arg(trailing_var_arg = true, allow_hyphen_values = true, hide = true)]
    args: Vec<String>,
}

impl NixArgs {
    fn run(&self, cmd: &str) -> Result<()> {
        let args: [&str; 1] = [ cmd ];
        let rest = self.args.iter().map(|s| &**s).collect::<Vec<&str>>();
        let new_args = args.iter().chain(rest.iter()).map(|s| *s).collect::<Vec<&str>>();
        Nix::exec(
            &new_args,
            &Workspace::find_root(&std::env::current_dir()?)
                .ok_or(anyhow!("Could not find workspace in current directory."))?,
        )?;
        Ok(())
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Commands::Init(cmd) => cmd.run(),
        Commands::Clone(cmd) => cmd.run(),
        Commands::Show(cmd) => cmd.run(),

        Commands::Config(cmd) => cmd.run(),
        Commands::Env(cmd) => cmd.run(),

        Commands::Attach(cmd) => cmd.run(),
        Commands::Detach(cmd) => cmd.run(),

        Commands::Link(cmd) => cmd.run(),
        Commands::Unlink(cmd) => cmd.run(),

        Commands::Sync(cmd) => cmd.run(),
        Commands::Publish(cmd) => cmd.run(),
        Commands::Update(cmd) => cmd.run(),

        Commands::Build(nix) => nix.run("build"),
        Commands::Run(nix) => nix.run("run"),
        Commands::Flake(nix) => nix.run("flake"),
    }?;
    Ok(())
}
