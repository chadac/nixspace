use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use anyhow::{anyhow, Error, Result};
use glob_match::glob_match;
use std::path::{Path, PathBuf};

use super::flake::{FlakeRef, InputSpec};
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
    pub fn update(&self, flake_ref: &FlakeRef) -> Result<Option<InputSpec>> {
        let remote_url = flake_ref.remote_url();
        if let Some(rev) = self.get_latest_rev(&remote_url)? {
            let flake_url = flake_ref.to_flake_url(Some(&rev));
            let hash = Nix::flake_prefetch(&flake_url)?.hash;
            Ok(Some(flake_ref.to_input_spec(&rev, &hash)))
        }
        else {
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
        flake_ref: &FlakeRef,
        path: &P,
        name: Option<String>,
    ) -> Result<&ProjectConfig> {
        let n = name.unwrap_or(
            flake_ref.repo().ok_or(
                anyhow!("could not infer a good project name to use.")
            )?
        );
        let mut pb = PathBuf::new();
        pb.push(path);
        self.projects.push(ProjectConfig {
            name: n.to_string(),
            url: flake_ref.url.to_string(),
            path: pb,
            strategy: None,
        });
        Ok(self.projects.last().unwrap())
    }

    pub fn rm_project(&mut self, flake_ref: &FlakeRef) -> Result<ProjectConfig> {
        let index = self.projects.iter().position(|x| x.url == flake_ref.url).ok_or(
            anyhow!("project with url '{}' not found", flake_ref.url)
        )?;
        Ok(self.projects.remove(index))
    }

    pub fn get_project_by_flake_ref(&self, flake_ref: &FlakeRef) -> Option<&ProjectConfig> {
        todo!()
    }
}

impl LocalConfig {
    pub fn new() -> Self {
        todo!()
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
                ("dev".to_string(), EnvConfig { strategy: UpdateStrategy::LATEST, }),
                ("stage".to_string(), EnvConfig { strategy: UpdateStrategy::FREEZE, }),
                ("prod".to_string(), EnvConfig {
                    strategy: UpdateStrategy::TAG(Some("release-*".to_string())),
                }),
            ]),
            projects: Vec::from([
                ProjectConfig {
                    url: "github:chadac/project-a".to_string(),
                    path: "./project-a".to_string(),
                    strategy: None,
                },
                ProjectConfig {
                    url: "github:chadac/project-b".to_string(),
                    path: "./subfolder/project-b".to_string(),
                    strategy: Some(HashMap::from([
                        ("stage".to_string(), UpdateStrategy::FREEZE),
                    ])),
                },
            ]),
        };
        let repr = serde_yaml::to_string(&config).unwrap();
        println!("{}", repr);
    }
}
