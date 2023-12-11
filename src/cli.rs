use std::path::{Path, PathBuf};
use std::process::{Command, Output, ExitStatus};
use serde::{Deserialize};
use anyhow::{anyhow, bail, Context, Result};

#[derive(Debug, Clone)]
pub struct CliError {
    cmd: String,
    args: Vec<String>,
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "cli error")
    }
}

pub struct CliOutput {
    stdout: String,
    stderr: String,
}

pub trait CliCommand {
    fn cmd() -> &'static str;

    fn run<P: AsRef<Path> + ?Sized>(
        args: &[&str],
        cwd: &P
    ) -> Result<ExitStatus> {
        let output = Command::new(Self::cmd())
            .args(args)
            .output()?;
        Ok(output.status)
    }

    fn exec<P: AsRef<Path> + ?Sized>(
        args: &[&str],
        cwd: &P
    ) -> Result<CliOutput> {
        let output = Command::new(Self::cmd())
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()?;
        let status = output.status;
        if status.success() {
            Ok(CliOutput {
                stdout: std::str::from_utf8(&output.stdout)?.to_string(),
                stderr: std::str::from_utf8(&output.stderr)?.to_string(),
            })
        } else {
            bail!(CliError {
                cmd: Self::cmd().to_string(),
                args: args.iter().map(|a| a.to_string()).collect(),
                status: status,
                stdout: std::str::from_utf8(&output.stdout)?.to_string(),
                stderr: std::str::from_utf8(&output.stderr)?.to_string(),
            })
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct FlakePrefetch {
    pub hash: String,
    #[serde(rename = "storePath")]
    pub store_path: String,
}

/// Minimal wrapper around the Nix CLI
pub struct Nix {}

/// Minimal wrapper around the Git CLI
pub struct Git {}

impl CliCommand for Nix {
    fn cmd() -> &'static str { "nix" }
}

impl Nix {
    pub fn clone<P1: AsRef<Path> + ?Sized, P2: AsRef<Path> + ?Sized>(flake_ref: &str, dest: &P1, cwd: &P2) -> Result<CliOutput> {
        Self::exec(
            &[
                "flake", "clone", flake_ref,
                "--dest", &dest.as_ref().as_os_str().to_str().unwrap()
            ],
            cwd
        )
    }

    /// Fetches the hash of a flake reference using `nix flake prefetch`
    pub fn flake_prefetch(flake_ref: &str) -> Result<FlakePrefetch> {
        let result = Self::exec(
            &["flake", "prefetch", flake_ref, "--json"],
            &std::env::current_dir()?
        )?;
        let out: FlakePrefetch = serde_json::from_str(&result.stdout)?;
        Ok(out)
    }
}

pub struct GitRef {
    pub rev: String,
    pub git_ref: String,
}

impl CliCommand for Git {
    fn cmd() -> &'static str { "git" }
}

fn get_git_context<P: AsRef<Path> + ?Sized>(path: &P) -> Result<(PathBuf, String)> {
    let path_abs = std::fs::canonicalize(&path)?;
    let git_root = crate::util::find_root(".git", &path_abs)
        .with_context(|| anyhow!("could not find .git folder in any parent directory"))?;

    let path_rel = path_abs.strip_prefix(git_root.clone())?.to_str()
        .context("path is not valid unicode and I'm lazy")?;

    Ok((git_root, path_rel.to_string()))
}

impl Git {
    pub fn init<P: AsRef<Path> + ?Sized>(cwd: &P) -> Result<CliOutput> {
        Self::exec(&["init"], cwd)
    }

    pub fn fetch<P: AsRef<Path> + ?Sized>(cwd: &P) -> Result<CliOutput> {
        Self::exec(&["fetch"], cwd)
    }

    pub fn push<P: AsRef<Path> + ?Sized>(cwd: &P) -> Result<CliOutput> {
        Self::exec(&["push", "origin"], cwd)
    }

    pub fn pull_rebase<P: AsRef<Path> + ?Sized>(cwd: &P) -> Result<CliOutput> {
        Self::exec(&["pull", "--rebase"], cwd)
    }

    /// Returns true if the file at the given path has been changed.
    pub fn changed<P: AsRef<Path> + ?Sized>(file_path: &P) -> Result<bool> {
        let (cwd, filename) = get_git_context(file_path)?;
        let s1 = Self::run(&["diff", "--exit-code", &filename], &cwd)?;
        if s1.success() {
            let s2 = Self::run(&["diff", "--staged", "--exit-code", &filename], &cwd)?;
            Ok(!s2.success())
        } else {
            Ok(true)
        }
    }

    pub fn add<P: AsRef<Path> + ?Sized>(file_path: &P) -> Result<CliOutput> {
        let (cwd, filename) = get_git_context(file_path)?;
        Self::exec(&["add", &filename], &cwd)
    }

    pub fn rm<P: AsRef<Path> + ?Sized>(file_path: &P) -> Result<CliOutput> {
        let (cwd, filename) = get_git_context(file_path)?;
        Self::exec(&["rm", "-r", "--cached", &filename], &cwd)
    }

    pub fn commit<P: AsRef<Path> + ?Sized>(message: &str, cwd: &P) -> Result<CliOutput> {
        Self::exec(&["commit", "-m", message], cwd)
    }

    pub fn reset<P: AsRef<Path> + ?Sized>(cwd: &P) -> Result<CliOutput> {
        Self::exec(&["reset"], cwd)
    }

    pub fn ls_remote(remote_url: &str) -> Result<Vec<GitRef>> {
        let result = Self::exec(
            &["ls-remote", "--sort='v:refname'", remote_url],
            &std::env::current_dir()?
        )?;
        let raw = result.stdout;
        let mut refs: Vec<GitRef> = Vec::new();
        for line in raw.split("\n") {
            let mut parts = line.split_whitespace();
            let rev = parts.next().ok_or(anyhow!("git ls-remote: unexpected input"))?;
            let git_ref = parts.next().ok_or(anyhow!("git ls-remote: unexpected input"))?;
            refs.push(GitRef {
                git_ref: git_ref.to_string(),
                rev: rev.to_string(),
            })
        }
        Ok(refs)
    }
}
