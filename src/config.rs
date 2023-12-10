use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use anyhow::{anyhow, Error, Result};
use glob_match::glob_match;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::flake::FlakeRef;
use super::lockfile::InputSpec;
use super::cli::{CliCommand, Git, Nix};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub environments: HashMap<String, EnvConfig>,
    pub projects: Vec<ProjectConfig>,

    pub default_env: String,
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum UpdateStrategy {
    #[serde(rename = "latest")]
    Latest,
    #[serde(rename = "freeze")]
    Freeze,
    #[serde(rename = "latest-tag")]
    LatestTag(Option<String>),
    #[serde(rename = "branch")]
    Branch(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EnvConfig {
    pub strategy: UpdateStrategy
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectConfig {
    pub name: String,
    pub url: String,
    pub path: PathBuf,
    pub strategy: Option<HashMap<String, UpdateStrategy>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LocalConfig {
    pub projects: HashMap<String, bool>
}

impl UpdateStrategy {
    pub fn update(&self, flake_ref: Rc<dyn FlakeRef>) -> Result<Option<InputSpec>> {
        if let Some(remote_url) = flake_ref.git_remote_url() {
            if let Some(rev) = self.get_latest_rev(&remote_url)? {
                let hash = Nix::flake_prefetch(&flake_ref.flake_url())?.hash;
                let mut input_spec = InputSpec::from_flake_ref(flake_ref);
                input_spec.rev = Some(rev);
                input_spec.nar_hash = Some(hash);
                Ok(Some(input_spec))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    fn get_latest_rev(&self, remote_url: &str) -> Result<Option<String>> {
        match self {
            Self::Latest => {
                let revs = Git::ls_remote(remote_url)?;
                Ok(Some(revs.iter()
                    .find(|r| r.git_ref == "HEAD")
                    .ok_or(Error::msg("could not find HEAD in repository"))?
                    .rev.clone()))
            },
            Self::Freeze => {
                Ok(None)
            },
            Self::LatestTag(pattern) => {
                let revs = Git::ls_remote(remote_url)?;
                let tag_pattern = match pattern {
                    Some(p) => p,
                    None => "*"
                };
                let glob = format!("refs/tags/{}", &tag_pattern);
                Ok(revs.iter()
                    .filter(|r| glob_match(&glob, &r.git_ref))
                    .map(|r| r.rev.clone())
                    .last())
            },
            Self::Branch(branch) => {
                let revs = Git::ls_remote(remote_url)?;
                let git_ref = format!("refs/branches/{}", branch);
                Ok(Some(
                    revs.iter()
                        .find(|r| r.git_ref == *branch)
                        .ok_or(Error::msg("could not find specified branch in repository"))?
                        .rev.clone()
                ))
            }
        }
    }
}

impl Config {
    pub fn new() -> Self {
        let mut default_envs = HashMap::new();
        default_envs.insert("dev".to_string(), EnvConfig {
            strategy: UpdateStrategy::Latest
        });
        Config {
            environments: default_envs,
            projects: Vec::new(),
            default_env: "dev".to_string(),
        }
    }

    pub fn read(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str::<Self>(&contents)?)
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        std::fs::write(path, serde_yaml::to_string(&self)?)?;
        Ok(())
    }

    pub fn environments(&self) -> Vec<String> {
        self.environments.keys().map(|name| name.to_string()).collect()
    }

    pub fn add_project<P: AsRef<Path> + ?Sized>(
        &mut self,
        flake_ref: &dyn FlakeRef,
        path: &P,
        name: Option<String>,
    ) -> Result<&ProjectConfig> {
        let n = name.unwrap_or(
            flake_ref.arg("repo").ok_or(
                anyhow!("could not infer a good project name to use.")
            )?
        );
        let mut pb = PathBuf::new();
        pb.push(path);
        self.projects.push(ProjectConfig {
            name: n.to_string(),
            url: flake_ref.flake_url(),
            path: pb,
            strategy: None,
        });
        Ok(self.projects.last().unwrap())
    }

    pub fn rm_project(&mut self, flake_ref: &dyn FlakeRef) -> Result<ProjectConfig> {
        let index = self.projects.iter().position(|x| x.url == flake_ref.flake_url()).ok_or(
            anyhow!("project with ref '{}' not found", flake_ref.flake_url())
        )?;
        Ok(self.projects.remove(index))
    }

    pub fn get_project_by_flake_ref(&self, flake_ref: Rc<dyn FlakeRef>) -> Option<&ProjectConfig> {
        todo!()
    }
}

impl LocalConfig {
    pub fn new() -> Self {
        LocalConfig {
            projects: HashMap::new(),
        }
    }

    pub fn read(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str::<Self>(&contents)?)
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        std::fs::write(path, serde_json::to_string(&self)?)?;
        Ok(())
    }

    /// Returns if a project is editable by the project name
    pub fn is_editable(&self, project_name: &str) -> bool {
        *self.projects.get(project_name).unwrap_or(&false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize() {
        let config = Config {
            environments: HashMap::from([
                ("dev".to_string(), EnvConfig { strategy: UpdateStrategy::Latest, }),
                ("stage".to_string(), EnvConfig { strategy: UpdateStrategy::Freeze, }),
                ("prod".to_string(), EnvConfig {
                    strategy: UpdateStrategy::LatestTag(Some("release-*".to_string())),
                }),
            ]),
            projects: Vec::from([
                ProjectConfig {
                    name: "project-a".to_string(),
                    url: "github:chadac/project-a".to_string(),
                    path: PathBuf::from("./project-a"),
                    strategy: None,
                },
                ProjectConfig {
                    name: "project-b".to_string(),
                    url: "github:chadac/project-b".to_string(),
                    path: PathBuf::from("./subfolder/project-b"),
                    strategy: Some(HashMap::from([
                        ("stage".to_string(), UpdateStrategy::Freeze),
                    ])),
                },
            ]),
            default_env: "dev".to_string(),
        };
        let repr = serde_yaml::to_string(&config).unwrap();
        println!("{}", repr);
    }
}
