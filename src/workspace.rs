use anyhow::{anyhow, bail, Error, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::flake::FlakeRef;
use super::lockfile::LockFile;
use super::config::{Config, LocalConfig, ProjectConfig};
use super::cli::{CliCommand, Git};

static CONFIG_PATH: &str = "nixspace.yml";
static LOCKFILE_DIR: &str = ".nixspace";
static LOCAL_PATH: &str = ".nixspace/nixspace.local";

pub struct Workspace {
    pub root: PathBuf,
    pub config: Config,
    pub lock: HashMap<String, LockFile>,
    pub local: LocalConfig,
}

#[derive(Clone)]
pub struct ProjectRef<'config> {
    pub flake_ref: Rc<dyn FlakeRef>,
    pub config: &'config ProjectConfig,
    pub editable: bool,
}

fn _config_path(root: &PathBuf) -> PathBuf {
    root.join(CONFIG_PATH)
}

fn _lock_path(root: &PathBuf, env: &str) -> PathBuf {
    root.join(LOCKFILE_DIR).join(format!("{}.lock", env))
}

fn _local_path(root: &PathBuf) -> PathBuf {
    root.join(LOCAL_PATH)
}

impl Workspace {
    pub fn discover() -> Result<Workspace> {
        let cwd = std::env::current_dir()?;
        let root = Self::find_root(&cwd).ok_or(anyhow!("Could not find workspace in current directory."))?;
        Self::at(&root)
    }

    pub fn init<P: AsRef<Path> + ?Sized>(root: &P) -> Result<Workspace> {
        let mut ns_root = PathBuf::new();
        ns_root.push(root);
        let config = Config::new();
        let envs = config.environments().clone();
        Ok(Workspace {
            root: ns_root,
            config: Config::new(),
            lock: envs.iter().map(|env| (env.to_string(), LockFile::empty())).collect(),
            local: LocalConfig::new(),
        })
    }

    pub fn at<P: AsRef<Path> + ?Sized>(path: &P) -> Result<Workspace> {
        let mut root = PathBuf::new();
        root.push(path);
        let config = Config::read(&_config_path(&root))?;
        let envs = config.environments().clone();
        Ok(Workspace {
            root: root.clone(),
            config: config,
            lock: envs.iter()
                .map(|env| {
                    let file = LockFile::read(&root.join(LOCKFILE_DIR).join(format!("{}.lock", env)));
                    match file {
                        Ok(f) => Ok((env.to_string(), f)),
                        Err(e) => Err(e),
                    }
                }).collect::<Result<HashMap<String, LockFile>, _>>()?,
            local: LocalConfig::read(&root.join(LOCAL_PATH))?,
        })
    }

    pub fn find_root<P: AsRef<Path> + ?Sized>(wd: &P) -> Option<PathBuf> {
        let mut cwd: PathBuf = PathBuf::new();
        let filename = "ws.yml";
        cwd.push(wd);
        loop {
            let path = cwd.as_path().join(filename);
            if path.exists() {
                return Some(cwd.as_path().into());
            }
            if !cwd.pop() {
                break
            }
        };
        None
    }

    pub fn config_path(&self) -> PathBuf {
        _config_path(&self.root)
    }

    pub fn lock_path(&self, env: &str) -> PathBuf {
        _lock_path(&self.root, env)
    }

    pub fn local_path(&self) -> PathBuf {
        _local_path(&self.root)
    }

    pub fn save(&self) -> Result<()> {
        self.config.write(&self.config_path())?;
        self.local.write(&self.local_path())?;
        for env in self.config.environments() {
            if let Some(lock) = self.lock.get(&env) {
                lock.write(&self.lock_path(&env))?;
            }
        }
        Ok(())
    }

    pub fn sync(&mut self) -> Result<()> {
        todo!()
    }

    pub fn tracks_latest(&self) -> Result<bool> {
        todo!()
    }

    /// pushes any new commits from the workspace
    pub fn publish(&self, force: bool) -> Result<()> {
        todo!()
    }

    pub fn project(&self, path_or_ref: &str) -> Result<Option<ProjectRef>> {
        let flake_ref = super::flake::parse(path_or_ref)?;
        if let Some(project_config) = self.config.get_project_by_flake_ref(flake_ref.clone()) {
            Ok(Some(ProjectRef {
                flake_ref: flake_ref,
                config: &project_config,
                editable: self.local.is_editable(&project_config.name),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn projects(&self) -> Vec<ProjectRef> {
        todo!()
    }

    pub fn add_project(&mut self, flake_ref: &str, directory: &str) -> Result<ProjectRef> {
        todo!()
    }

    pub fn remove_project(&mut self, path_or_ref: &str) -> Result<ProjectRef> {
        todo!()
    }

    pub fn mark_editable(&mut self, project_name: &str) -> () {
        self.local.projects.insert(project_name.to_string(), true);
    }

    pub fn unmark_editable(&mut self, project_name: &str) -> () {
        self.local.projects.insert(project_name.to_string(), false);
    }

    pub fn update_lock(&mut self, env: &Option<String>, projects: &Vec<String>) -> Result<()> {
        let e: String = match env {
            Some(v) => v.to_string(),
            None => self.config.default_env.to_string(),
        };
        self.lock.get(&e).ok_or(
            anyhow!("error: workspace config missing env '{}'", e)
        )?;

        let default = self.config.environments.get(&e)
            .expect("missing environment")
            .strategy.clone();
        let mut lock_updates = Vec::new();
        for project in self.projects() {
            let strategy = {
                match &project.config.strategy {
                    Some(cfg) => cfg.get(&e).unwrap_or(&default),
                    None => &default,
                }
            };
            if let Some(input_spec) = strategy.update(project.flake_ref)? {
                lock_updates.push((project.config.name.to_string(), input_spec));
            } else {
                // TODO: Log skipped input here
            }
        }
        let lock = self.lock.get_mut(&e).unwrap();
        for (project_name, input_spec) in lock_updates {
            lock.update(&project_name, &input_spec)?;
        }
        Ok(())
    }

    /// Tracks all local editable projects in the Git repository.
    ///
    /// Necessary since Nix Flakes only track projects which are
    /// tracked in Git.
    pub fn stage_editable_projects(&self) -> Result<()> {
        for project in self.projects().iter() {
            if project.editable {
                Git::add(&project.config.path)?;
            }
        }
        Ok(())
    }

    /// Creates a commit tracking the config and lockfile.
    pub fn commit(&self, commit_message: &str) -> Result<()> {
        Git::reset(&self.root)?;
        Git::add(&self.config_path())?;
        for env in self.config.environments() {
            Git::add(&self.lock_path(&env))?;
        }
        Git::commit(commit_message, &self.root)?;
        self.stage_editable_projects()?;
        Ok(())
    }
}

impl<'config> ProjectRef<'config> {
    pub fn sync(&self) -> Result<()> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use tempdir::TempDir;
    use super::Workspace;

    #[test]
    fn finds_root_works() -> Result<()> {
        let tmp = TempDir::new("workspace")?;
        let cwd = tmp.path().join("a/b/c/d/e");
        std::fs::create_dir_all(cwd.clone())?;
        let ws = tmp.path().join("a/b/ws.yml");
        std::fs::OpenOptions::new().create(true).write(true).open(ws.clone())?;
        let root = Workspace::find_root(&cwd);
        assert_eq!(root, Some(ws.parent().unwrap().into()));
        Ok(())
    }

    #[test]
    fn find_root_fails_not_in_ws() -> Result<()> {
        let tmp = TempDir::new("workspace")?;
        let cwd = tmp.path().join("a/b/c/d/e");
        std::fs::create_dir_all(cwd.clone())?;
        assert!(
            Workspace::find_root(&cwd).is_none()
        );
        Ok(())
    }
}
