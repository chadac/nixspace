use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use serde::{Serialize, Deserialize};

use super::config::UpdateStrategy;


#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum FlakeType {
    #[serde(rename = "path")]
    Path,
    #[serde(rename = "git+https")]
    GitHTTPS,
    #[serde(rename = "git+ssh")]
    GitSSH,
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
    #[serde(rename = "indirect")]
    Indirect,
}

#[derive(Clone, PartialEq, Debug)]
pub struct FlakeRef {
    pub flake_type: FlakeType,
    pub url: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct InputSpec {
    #[serde(rename = "type")]
    pub flake_type: FlakeType,
    #[serde(rename = "narHash")]
    pub nar_hash: Option<String>,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub dir: Option<String>,
    pub rev: Option<String>,
    #[serde(rename = "ref")]
    pub flake_ref: Option<String>,
    #[serde(rename = "revCount")]
    pub rev_count: Option<i64>,
    #[serde(rename = "lastModified")]
    pub last_modified: Option<i64>,
}

impl FlakeRef {
    pub fn parse(path_or_ref: &str) -> Result<FlakeRef> {
        match Self::parse_ref(path_or_ref) {
            Ok(f) => Ok(f),
            Err(_) => Ok(Self::parse_path(path_or_ref)?)
        }
    }

    pub fn parse_ref(flake_ref: &str) -> Result<FlakeRef> {
        use serde::de::IntoDeserializer;
        use serde::de::value::Error;

        let flake_ref_parts = flake_ref
            .split(":")
            .collect::<Vec<&str>>();
        let (flake_type_parts, url_parts) = flake_ref_parts.split_at(1);
        let flake_type: Result<FlakeType, Error> = FlakeType::deserialize(flake_type_parts[0].into_deserializer());
        let url = url_parts.join(":");
        Ok(FlakeRef {
            flake_type: flake_type?,
            url: url,
        })
    }

    pub fn parse_path<P: AsRef<Path> + ?Sized>(path: &P) -> Result<FlakeRef> {
        todo!()
    }

    pub fn args(&self) -> HashMap<String, String> {
        todo!()
    }

    pub fn remote_url(&self) -> String {
        todo!()
    }

    pub fn to_flake_url(&self, rev: Option<&str>) -> String {
        todo!()
    }

    pub fn to_input_spec(&self, rev: &str, nar_hash: &str) -> InputSpec {
        todo!()
    }

    pub fn repo(&self) -> Option<String> {
        todo!()
    }
}

impl InputSpec {
    /// Returns an updated version of an InputSpec based on the given update strategy
    fn update(&self, strategy: UpdateStrategy) -> InputSpec {
        todo!()
    }

    /// Returns an InputSpec based on a given revision.
    pub fn parse(flake_ref: &str) -> Result<InputSpec> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use super::{FlakeRef, FlakeType};

    #[test]
    fn flake_ref_parses() -> Result<()> {
        assert_eq!(
            FlakeRef::parse("github:NixOS/nixpkgs")?,
            FlakeRef {
                flake_type: FlakeType::GITHUB,
                url: "NixOS/nixpkgs".to_string(),
            }
        );
        assert_eq!(
            FlakeRef::parse("path:/nix/store/bash")?,
            FlakeRef {
                flake_type: FlakeType::PATH,
                url: "/nix/store/bash".to_string(),
            }
        );
        Ok(())
    }
}
