#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod workspace;
mod config;
mod lockfile;
mod flake;
mod cli;
mod util;

use crate::config::Config;
use crate::cli::{CliCommand, Git, Nix};
use crate::workspace::{ProjectRef, Workspace};
use crate::flake::FlakeRef;
use crate::lockfile::InputSpec;

use anyhow::{anyhow, bail, Context, Error, Result};
use clap::{Args, Parser, Subcommand};
use std::path::Path;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
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
    Register(Register),
    /// erase a project from the workspace.
    Unregister(Unregister),

    // LOCAL PROJECT COMMANDS
    /// link a project locally into the workspace;
    /// i.e., clone it and make it editable.
    Edit(Edit),
    /// unlink the project from the workspace
    Unedit(Unedit),

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
        let flake_ref = flake::parse(&self.flake_ref)?;
        let input_spec = InputSpec::from_flake_ref(flake_ref);
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
        let ws = Workspace::discover()?;
        ws.print_tree();
        Ok(())
    }
}

#[derive(Debug, Subcommand)]
enum ConfigSubcommand {
    /// get a configuration value
    Get(ConfigGet),
    /// set a configuration value
    Set(ConfigSet),
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

impl Command for ConfigSubcommand {
    fn run(&self) -> Result<()> {
        match &self {
            ConfigSubcommand::Get(get) => {
                let ws = Workspace::discover()?;
                match get.name.as_str() {
                    "default_env" => println!("{}", toml::to_string(&ws.config.default_env)?),
                    _ => bail!("error: unrecognized configuration name {}", get.name),
                };
            },
            ConfigSubcommand::Set(set) => {
                let mut ws = Workspace::discover()?;
                match set.name.as_str() {
                    "default_env" => {
                        ws.config.default_env = set.value.to_string();
                    }
                    _ => bail!("error: unrecognized configuration name {}", set.name),
                }
                ws.save()?;
            },
        }
        Ok(())
    }
}

#[derive(Debug, Subcommand)]
enum EnvSubcommand {
    /// get a configuration value
    Get(EnvGet),
    /// set a configuration value
    Set(EnvSet),
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

impl Command for EnvSubcommand {
    fn run(&self) -> Result<()> {
        match &self {
            EnvSubcommand::Get(get) => {
                let ws = Workspace::discover()?;
                let env = ws.config.env(&get.env)?;
                match get.name.as_str() {
                    // todo: serialize
                    "strategy" => println!("{}", serde_json::to_string(&env.strategy)?),
                    _ => bail!("error: unrecognized environment key '{}'", get.name),
                }
            },
            EnvSubcommand::Set(set) => {
                let mut ws = Workspace::discover()?;
                let env = ws.config.env_mut(&set.env)?;
                match set.name.as_str() {
                    "strategy" => {
                        env.strategy = serde_json::from_str(&set.value)?;
                    },
                    _ => bail!("error: unrecognized environment key '{}'", set.name),
                }
                ws.save()?;
            },
        };
        Ok(())
    }
}

#[derive(Args, Debug)]
struct Register {
    /// path or reference to the project.
    path_or_ref: String,
    /// name of the directory that the project will be cloned into when added.
    /// default is the name of the project at the root of the workspace.
    #[arg(short, long)]
    path: Option<String>,
    /// name of the project used for replacing in flake.nix files
    /// default is the name of the project (if it can be inferred)
    #[arg(short, long)]
    name: Option<String>,
    /// if present, clones the project locally
    #[arg(long)]
    edit: bool,
}

impl Command for Register {
    fn run(&self) -> Result<()> {
        let mut ws = Workspace::discover()?;
        let flake_ref = flake::parse(&self.path_or_ref)?;
        let name = self.name.as_ref().map(|s| s.to_string()).unwrap_or(
            flake_ref.infer_name().context("could not infer project name!")?
        );
        let project = ws.register(&name, flake_ref, &self.path)?;

        if self.edit {
            ws.edit(&name)?;
        }

        ws.save()?;
        Ok(())
    }
}

#[derive(Args, Debug)]
struct Unregister {
    /// identifier for the project
    name: String,
    #[arg(long)]
    /// if present, delete the directory from the workspace
    delete: bool,
}

impl Command for Unregister {
    fn run(&self) -> Result<()> {
        let mut ws = Workspace::discover()?;
        ws.deregister(&self.name, self.delete)?;
        ws.save()?;
        Ok(())
    }
}

#[derive(Args, Debug)]
struct Edit {
    /// name of the project
    name: String,
}

impl Command for Edit {
    fn run(&self) -> Result<()> {
        let mut ws = Workspace::discover()?;
        ws.edit(&self.name)?;
        ws.save()?;
        Ok(())
    }
}

#[derive(Args, Debug)]
struct Unedit {
    /// name of the project
    name: String,
    /// if present, deletes the project locally
    #[arg(long)]
    rm: bool
}

impl Command for Unedit {
    fn run(&self) -> Result<()> {
        let mut ws = Workspace::discover()?;
        ws.unedit(&self.name, self.rm)?;
        ws.save()?;
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
    /// environment to update
    env: Option<String>,
    /// if present, publishes the new lockfile to the Git repository
    #[arg(long)]
    publish: bool,
}

impl Command for Update {
    fn run(&self) -> Result<()> {
        let mut ws = Workspace::discover()?;
        ws.update_all_projects(&self.env)?;
        ws.save()?;
        if self.publish {
            if ws.tracks_latest()? {
                ws.commit("chore: update workspace")?;
                ws.publish(false)?;
            }
            else {
                bail!("cannot commit; upstream is ahead of local git repository.");
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

fn exec(command: &Commands) -> Result<()> {
    match command {
        Commands::Init(cmd) => cmd.run(),
        Commands::Clone(cmd) => cmd.run(),
        Commands::Show(cmd) => cmd.run(),

        Commands::Config(cmd) => cmd.run(),
        Commands::Env(cmd) => cmd.run(),

        Commands::Register(cmd) => cmd.run(),
        Commands::Unregister(cmd) => cmd.run(),

        Commands::Edit(cmd) => cmd.run(),
        Commands::Unedit(cmd) => cmd.run(),

        Commands::Sync(cmd) => cmd.run(),
        Commands::Publish(cmd) => cmd.run(),
        Commands::Update(cmd) => cmd.run(),

        Commands::Build(nix) => nix.run("build"),
        Commands::Run(nix) => nix.run("run"),
    }?;
    Ok(())
}

fn main() -> () {
    let cli = Cli::parse();
    if let Some(v) = cli.verbose.log_level() {
        let filter = v.to_level_filter();
        let config = simplelog::ConfigBuilder::new()
            .set_time_level(log::LevelFilter::Off)
            .set_thread_level(log::LevelFilter::Off)
            .build();

        simplelog::CombinedLogger::init(
            vec![
                simplelog::TermLogger::new(filter, config, simplelog::TerminalMode::Mixed, simplelog::ColorChoice::Auto),
            ]
        ).unwrap();

        // capture the backtrace if we're at trace level
        if v >= log::Level::Trace {
            std::env::set_var("RUST_BACKTRACE", "1");
        }
    }

    match exec(&cli.command) {
        Ok(()) => (),
        Err(e) => {
            log::error!("{e}");
            log::trace!("backtrace:\n{}", e.backtrace());
            std::process::exit(0x0100);
        },
    }
}
