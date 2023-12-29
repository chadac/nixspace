use serde::{Serialize, Deserialize};
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use super::flake::FlakeRef;

#[derive(Serialize, Deserialize, Debug)]
pub struct LockFile {
    nodes: HashMap<String, LockedRef>,
    root: String,
    version: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct LockedRef {
    locked: Option<InputSpec>,
    original: Option<InputSpec>,
    inputs: Option<HashMap<String, String>>,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum FlakeType {
    #[serde(rename = "path")]
    Path,
    #[serde(rename = "git")]
    Git,
    #[serde(rename = "mercurial")]
    Mercurial,
    #[serde(rename = "tarball")]
    Tarball,
    #[serde(rename = "file")]
    File,
    #[serde(rename = "github")]
    GitHub,
    #[serde(rename = "gitlab")]
    GitLab,
    #[serde(rename = "sourcehut")]
    SourceHut,
    #[serde(rename = "flake")]
    Indirect,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct InputSpec {
    #[serde(rename = "type")]
    pub flake_type: FlakeType,
    #[serde(rename = "narHash")]
    pub nar_hash: Option<String>,
    pub url: Option<String>,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub dir: Option<String>,
    pub rev: Option<String>,
    #[serde(rename = "ref")]
    pub git_ref: Option<String>,
    #[serde(rename = "revCount")]
    pub rev_count: Option<i64>,
    #[serde(rename = "lastModified")]
    pub last_modified: Option<i64>,
}

impl LockFile {
    /// Generate an empty lockfile
    pub fn empty() -> Self {
        Self {
            nodes: HashMap::from([
                ("root".to_string(),
                 LockedRef {
                     locked: None,
                     original: None,
                     inputs: Some(HashMap::new()),
                 }),
            ]),
            root: "root".to_string(),
            version: 7,
        }
    }

    pub fn read(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str::<LockFile>(&contents)?)
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        std::fs::write(path, serde_json::to_string(&self)?)?;
        Ok(())
    }

    pub fn get_input_spec(&self, name: &str) -> Option<InputSpec> {
        self.nodes.get(name).map(|r| r.locked.clone()).flatten()
    }

    pub fn add(&mut self, name: &str, new_input_spec: &InputSpec) -> Result<()> {
        let p = self.nodes.get_mut(name)
            .ok_or(anyhow!("could not find project '{}'", name))?;
        p.locked = Some(new_input_spec.clone());
        let root = self.nodes.get_mut("root")
            .context("failed parsing lockfile: missing entry 'root' in nodes")?;
        if let Some(ref mut inputs) = root.inputs {
            inputs.insert(name.to_string(), name.to_string());
        }
        Ok(())
    }

    pub fn rm(&mut self, name: &str) -> Result<()> {
        self.nodes.remove(name);
        let root = self.nodes.get_mut("root")
            .context("failed parsing lockfile; missing entry 'root' in nodes")?;
        if let Some(ref mut inputs) = root.inputs {
            inputs.remove(name);
        }
        Ok(())
    }
}

impl InputSpec {
    pub fn from_flake_ref(flake_ref: Rc<dyn FlakeRef>) -> Self {
        flake_ref.input_spec()
        // let args = flake_ref.args().into_iter().collect::<HashMap<String, String>>();
        // InputSpec {
        //     flake_type: flake_ref.flake_type.clone(),
        //     nar_hash: None,
        //     url: Some(flake_ref.url().to_string()),
        //     owner: flake_ref.owner(),
        //     repo: flake_ref.repo(),
        //     dir: flake_ref.arg("dir"),
        //     rev: flake_ref.arg("rev"),
        //     git_ref: Some(flake_ref.arg("ref").unwrap_or("HEAD".to_string())),
        //     rev_count: None,
        //     last_modified: None,
        // }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lockfile_add_succeeds() -> Result<()> {

        Ok(())
    }
}
