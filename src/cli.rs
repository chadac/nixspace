use std::path::Path;
use std::process::{Command, Output};
use serde::{Deserialize};
use anyhow::{anyhow, Result};

#[derive(Debug, Clone)]
pub struct CliError {
    cmd: String,
    args: Vec<String>,
}

pub trait CliCommand {
    fn cmd() -> &'static str;

    fn exec<P: AsRef<Path> + ?Sized>(
        args: &[&str],
        cwd: &P
    ) -> Result<Output> {
        let output = Command::new(Self::cmd())
            .args(args)
            .output();
        // match output {
        //     Ok(r) => Ok(r),
        //     Err(e) => CliError { cmd: self.cmd, args: args },
        // }
        todo!()
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
    pub fn clone<P1: AsRef<Path> + ?Sized, P2: AsRef<Path> + ?Sized>(flake_ref: &str, dest: &P1, cwd: &P2) -> Result<i32> {
        let result = Self::exec(&[
            "flake", "clone", flake_ref,
            "--dest", &dest.as_ref().as_os_str().to_str().unwrap()
        ], cwd)?;
        Ok(result.status.code().unwrap())
    }

    /// Fetches the hash of a flake reference using `nix flake prefetch`
    pub fn flake_prefetch(flake_ref: &str) -> Result<FlakePrefetch> {
        let result = Self::exec(
            &["flake", "prefetch", flake_ref, "--json"],
            &std::env::current_dir()?
        )?;
        let out: FlakePrefetch = serde_json::from_str(
            &std::str::from_utf8(&result.stdout)?
        )?;
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

impl Git {
    pub fn init<P: AsRef<Path> + ?Sized>(cwd: &P) -> Result<Output> {
        Self::exec(&["init"], cwd)
    }

    pub fn fetch<P: AsRef<Path> + ?Sized>(cwd: &P) -> Result<Output> {
        Self::exec(&["fetch"], cwd)
    }

    pub fn push<P: AsRef<Path> + ?Sized>(cwd: &P) -> Result<Output> {
        Self::exec(&["push", "origin"], cwd)
    }

    pub fn track_head<P: AsRef<Path> + ?Sized>(cwd: &P) -> Result<()> {
        todo!()
    }

    /// Returns true if the file at the given path has been changed.
    pub fn changed<P: AsRef<Path> + ?Sized>(file_path: &P) -> Result<bool> {
        todo!()
    }

    pub fn add<P: AsRef<Path> + ?Sized>(file_path: &P) -> Result<()> {
        todo!()
    }

    pub fn rm<P: AsRef<Path> + ?Sized>(file_path: &P) -> Result<()> {
        todo!()
    }

    pub fn commit<P: AsRef<Path> + ?Sized>(message: &str, cwd: &P) -> Result<()> {
        todo!()
    }

    pub fn reset<P: AsRef<Path> + ?Sized>(cwd: &P) -> Result<()> {
        todo!()
    }

    pub fn ls_remote(remote_url: &str) -> Result<Vec<GitRef>> {
        let result = Self::exec(
            &["ls-remote", "--sort='v:refname'", remote_url],
            &std::env::current_dir()?
        )?;
        let raw = std::str::from_utf8(&result.stdout)?;
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
