use serde::{Serialize, Deserialize};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::Path;

use super::flake::InputSpec;

#[derive(Serialize, Deserialize, Debug)]
pub struct LockFile {
    environments: HashMap<String, FlakeLock>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FlakeLock {
    nodes: HashMap<String, LockedRef>,
    root: String,
    version: i32,
}


#[derive(Serialize, Deserialize, Debug)]
struct LockedRef {
    locked: InputSpec,
    original: InputSpec,
    inputs: HashMap<String, String>,
}


impl LockFile {
    pub fn empty() -> Self {
        todo!()
    }

    pub fn read(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str::<LockFile>(&contents)?)
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        std::fs::write(path, serde_json::to_string(&self)?)?;
        Ok(())
    }

    pub fn get_input_spec(&self, name: &str) -> InputSpec {
        todo!()
   }

    pub fn update(&mut self, env: &str, name: &str, new_input_spec: &InputSpec) -> Result<()> {
        let e = self.environments.get_mut(env)
            .ok_or(anyhow!("could not find environment '{}'", env))?;
        let p = e.nodes.get_mut(name)
            .ok_or(anyhow!("could not find project '{}'", name))?;
        p.locked = new_input_spec.clone();
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
}
