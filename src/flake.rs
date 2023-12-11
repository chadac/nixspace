use anyhow::{bail, Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use serde::{Serialize, Deserialize};
use serde_variant::to_variant_name;
use querystring::{querify, stringify};

use super::config::UpdateStrategy;
use crate::lockfile::{FlakeType, InputSpec};

fn split_once(msg: &str, split: &str) -> Result<(String, String)> {
    let mut parts = msg.split(split);
    let first = parts.next().context("failed to parse")?;
    let rest = parts.fold(String::new(), |a, b| a + b);
    Ok((first.to_string(), rest))
}

/// splits <scheme>:<url> into a (<scheme>, <url>) tuple
fn split_scheme(url: &str) -> Result<(String, String)> {
    split_once(url, ":")
}

pub trait FlakeRef {
    fn flake_url(&self) -> String;
    fn flake_type(&self) -> FlakeType;
    fn git_remote_url(&self) -> Option<String>;
    fn arg(&self, arg: &str) -> Option<String>;

    /// try to infer a name for the flake
    fn infer_name(&self) -> Option<String> {
        self.arg("repo")
    }

    fn input_spec(&self) -> InputSpec {
        InputSpec {
            flake_type: self.flake_type(),
            nar_hash: None,
            url: Some(self.flake_url()),
            owner: self.arg("owner"),
            repo: self.arg("repo"),
            dir: self.arg("dir"),
            rev: self.arg("rev"),
            git_ref: self.arg("ref"),
            rev_count: None,
            last_modified: None,
        }
    }
}

pub fn parse(url: &str) -> Result<Rc<dyn FlakeRef>> {
    let (scheme, url) = split_scheme(url)?;
    let result: Rc<dyn FlakeRef> = match (scheme.as_str(), url) {
        ("flake", rest) => FlakeIndirect::parse(&rest)?,
        ("path", rest) => FlakePath::parse(&rest)?,
        ("git+http", rest) => GitUrl::parse("http", &rest)?,
        ("git+https", rest) => GitUrl::parse("https", &rest)?,
        ("git+ssh", rest) => GitUrl::parse("ssh", &rest)?,
        ("git+file", rest) => GitUrl::parse("file", &rest)?,
        ("mc+http", rest) => MercurialUrl::parse("http", &rest)?,
        ("mc+https", rest) => MercurialUrl::parse("https", &rest)?,
        ("mc+ssh", rest) => MercurialUrl::parse("ssh", &rest)?,
        ("mc+file", rest) => MercurialUrl::parse("file", &rest)?,
        ("tarball+http", rest) => TarballUrl::parse("http", &rest)?,
        ("tarball+https", rest) => TarballUrl::parse("https", &rest)?,
        ("tarball+file", rest) => TarballUrl::parse("file", &rest)?,
        ("github", rest) => SimpleGitUrl::parse("github", "github.com/", &rest)?,
        ("gitlab", rest) => SimpleGitUrl::parse("gitlab", "gitlab.com/", &rest)?,
        ("sourcehut", rest) => SimpleGitUrl::parse("sourcehut", "git.sr.ht/~", &rest)?,
        (scheme, _) => bail!("unrecognized flake scheme: '{}'", scheme)
    };
    Ok(result)
}

/// format:
/// [flake:]<flake-id>(/<rev-or-ref>(/rev)?)?
#[derive(Clone, PartialEq, Debug)]
pub struct FlakeIndirect {
    flake_id: String,
    rev_or_ref: Option<String>,
    rev: Option<String>
}

impl FlakeIndirect {
    fn parse(url: &str) -> Result<Rc<dyn FlakeRef>> {
        let re = Regex::new("([^/]+)(?:/([^/]+)(?:/([^/]+))?)?")?;
        let m = re.captures(url).context("failed to parse indirect flake url")?;
        Ok(Rc::new(Self {
            flake_id: m.get(1).unwrap().as_str().to_string(),
            rev_or_ref: m.get(2).map(|s| s.as_str().to_string()),
            rev: m.get(3).map(|s| s.as_str().to_string()),
        }))
    }
}

impl FlakeRef for FlakeIndirect {
    fn flake_url(&self) -> String {
        format!(
            "flake:{flake_id}{rev_or_ref}{rev}",
            flake_id=self.flake_id,
            rev_or_ref=self.rev_or_ref.as_ref().map(|s| format!("/{}", s)).unwrap_or("".to_string()),
            rev=self.rev.as_ref().map(|s| format!("/{}", s)).unwrap_or("".to_string())
        )
    }

    fn flake_type(&self) -> FlakeType {
        FlakeType::Indirect
    }

    fn git_remote_url(&self) -> Option<String> {
        None
    }

    fn arg(&self, arg: &str) -> Option<String> {
        match arg {
            "ref" => self.rev_or_ref.clone(),
            "rev" => self.rev.clone(),
            _ => None
        }
    }
}

/// format:
/// path:<path>(\?<params>)?
#[derive(Clone, PartialEq, Debug)]
pub struct FlakePath {
    path: String,
    params: Vec<(String, String)>,
}

fn qs_to_ref(qs: &Vec<(String, String)>) -> Vec<(&str, &str)> {
    qs.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect()
}

fn params_to_string(params: &Vec<(String, String)>) -> String {
    if params.is_empty() {
        "".to_string()
    } else {
        let mut params = stringify(qs_to_ref(&params));
        // remove trailing '&' in path string
        params.pop();
        format!("?{}", params)
    }
}

impl FlakePath {
    pub fn parse(url: &str) -> Result<Rc<dyn FlakeRef>> {
        let re = Regex::new("([^?]+)(?:[?](.+))?$")?;
        let m = re.captures(url).context("failed to parse flake path")?;
        Ok(Rc::new(FlakePath {
            path: m.get(1).unwrap().as_str().to_string(),
            params: querify(m.get(2).map_or("", |s| s.as_str()))
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }))
    }
}

impl FlakeRef for FlakePath {
    fn flake_url(&self) -> String {
        format!(
            "path:{path}{params}",
            path=self.path,
            params=params_to_string(&self.params),
        )
    }
    fn flake_type(&self) -> FlakeType {
        FlakeType::Path
    }
    fn git_remote_url(&self) -> Option<String> {
        None
    }
    fn arg(&self, arg: &str) -> Option<String> {
        self.params.iter().find(|(k, _)| k == arg).map(|(_, v)| v.to_string())
    }
}

fn parse_server_url(url: &str) -> Result<(Option<String>, String, Vec<(String, String)>)> {
    let re = Regex::new("(?://([^/]+))?([^?]+)(?:[?](.+))?")?;
    let m = re.captures(url).with_context(|| format!("failed to parse server url {url}"))?;
    Ok((
        m.get(1).map(|s| s.as_str().to_string()),
        m.get(2).unwrap().as_str().to_string(),
        querify(m.get(3).map_or("", |s| s.as_str()))
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    ))
}

/// format:
/// git(+http|+https|+ssh|+git|+file):(//<server>)?<path>(\?<params>)?
#[derive(Clone, PartialEq, Debug)]
pub struct GitUrl {
    scheme: String,
    server: Option<String>,
    path: String,
    params: Vec<(String, String)>
}

impl GitUrl {
    pub fn parse(scheme: &str, url: &str) -> Result<Rc<dyn FlakeRef>> {
        let (server, path, params) = parse_server_url(url)?;
        Ok(Rc::new(GitUrl {
            scheme: scheme.to_string(),
            server: server,
            path: path,
            params: params,
        }))
    }
}

impl FlakeRef for GitUrl {
    fn flake_url(&self) -> String {
        format!(
            "git+{scheme}:{server}{path}{params}",
            scheme=self.scheme,
            server=self.server.as_ref().map_or("".to_string(), |s| format!("//{s}")),
            path=self.path,
            params=params_to_string(&self.params),
        )
    }
    fn flake_type(&self) -> FlakeType {
        FlakeType::Git
    }
    fn git_remote_url(&self) -> Option<String> {
        Some(format!(
            "{scheme}:{server}{path}",
            scheme=self.scheme,
            server=self.server.as_ref().map_or("".to_string(), |s| format!("//{s}")),
            path=self.path
        ))
    }
    fn arg(&self, arg: &str) -> Option<String> {
        self.params.iter().find(|(k, _)| k == arg).map(|(_, v)| v.to_string())
    }
}

/// format:
/// mc(+http|+https|+ssh|+file):(//<server>)?<path>(\?<params>)?
#[derive(Clone, PartialEq, Debug)]
pub struct MercurialUrl {
    scheme: String,
    server: Option<String>,
    path: String,
    params: Vec<(String, String)>
}

impl MercurialUrl {
    pub fn parse(scheme: &str, url: &str) -> Result<Rc<dyn FlakeRef>> {
        let (server, path, params) = parse_server_url(url)?;
        Ok(Rc::new(MercurialUrl {
            scheme: scheme.to_string(),
            server: server,
            path: path,
            params: params,
        }))
    }
}

impl FlakeRef for MercurialUrl {
    fn flake_url(&self) -> String {
        format!(
            "mc+{scheme}:{server}{path}{params}",
            scheme=self.scheme,
            server=self.server.as_ref().map_or("".to_string(), |s| format!("//{s}")),
            path=self.path,
            params=params_to_string(&self.params),
        )
    }
    fn flake_type(&self) -> FlakeType {
        FlakeType::Mercurial
    }
    fn git_remote_url(&self) -> Option<String> {
        None
    }
    fn arg(&self, arg: &str) -> Option<String> {
        self.params.iter().find(|(k, _)| k == arg).map(|(_, v)| v.to_string())
    }
}

/// format:
/// tarball(+http|+https|file)://<url>
#[derive(Clone, PartialEq, Debug)]
pub struct TarballUrl {
    scheme: String,
    url: String,
}

impl TarballUrl {
    pub fn parse(scheme: &str, url: &str) -> Result<Rc<dyn FlakeRef>> {
        let re = Regex::new("//(.+)")?;
        let m = re.captures(url).with_context(|| format!("could not parse tarball url: '{url}'"))?;
        Ok(Rc::new(TarballUrl {
            scheme: scheme.to_string(),
            url: m.get(1).map(|s| s.as_str().to_string()).unwrap(),
        }))
    }
}

impl FlakeRef for TarballUrl {
    fn flake_url(&self) -> String {
        format!(
            "tarball+{scheme}://{url}",
            scheme=self.scheme,
            url=self.url,
        )
    }
    fn flake_type(&self) -> FlakeType {
        FlakeType::Tarball
    }
    fn git_remote_url(&self) -> Option<String> {
        None
    }
    fn arg(&self, arg: &str) -> Option<String> {
        None
    }
}

fn parse_simple_url(url: &str) -> Result<(String, String, Option<String>, Vec<(String, String)>)> {
    let re = Regex::new("([^/]+)/([^/]+)(?:/([^?]+))?(?:[?](.+))?")?;
    let m = re.captures(url).with_context(|| format!("could not parse simple url: '{url}'"))?;
    Ok((
        m.get(1).map(|s| s.as_str().to_string()).unwrap(),
        m.get(2).map(|s| s.as_str().to_string()).unwrap(),
        m.get(3).map(|s| s.as_str().to_string()),
        querify(m.get(4).map_or("", |s| s.as_str()))
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    ))
}

fn fmt_simple_url(scheme: &str, owner: &str, repo: &str, rev_or_ref: &Option<String>, params: &Vec<(String, String)>) -> String {
    format!(
        "{scheme}:{owner}/{repo}{rev_or_ref}{params}",
        rev_or_ref=rev_or_ref.as_ref().map_or("".to_string(), |s| format!("/{s}")),
        params=params_to_string(params),
    )
}

/// format:
/// (github|gitlab|sourcehut):<owner>/<repo>(/<rev-or-ref>)?(\?<params>)?
#[derive(Clone, PartialEq, Debug)]
pub struct SimpleGitUrl {
    scheme: String,
    domain: String,
    owner: String,
    repo: String,
    rev_or_ref: Option<String>,
    params: Vec<(String, String)>,
}

impl SimpleGitUrl {
    pub fn parse(scheme: &str, domain: &str, url: &str) -> Result<Rc<dyn FlakeRef>> {
        let (owner, repo, rev_or_ref, params) = parse_simple_url(url)?;
        Ok(Rc::new(Self {
            scheme: scheme.to_string(),
            domain: domain.to_string(),
            owner: owner,
            repo: repo,
            rev_or_ref: rev_or_ref,
            params: params
        }))
    }
}

impl FlakeRef for SimpleGitUrl {
    fn flake_url(&self) -> String {
        fmt_simple_url("github", &self.owner, &self.repo, &self.rev_or_ref, &self.params)
    }
    fn flake_type(&self) -> FlakeType {
        FlakeType::GitHub
    }
    fn git_remote_url(&self) -> Option<String> {
        Some(format!(
            "https://{domain}{owner}/{repo}.git",
            domain=self.domain,
            owner=self.owner,
            repo=self.repo,
        ))
    }
    fn arg(&self, arg: &str) -> Option<String> {
        match arg {
            "owner" => Some(self.owner.to_string()),
            "repo" => Some(self.repo.to_string()),
            "rev_or_ref" => self.rev_or_ref.clone(),
            _ => None
        }
    }
}


#[cfg(test)]
mod tests {
    use anyhow::Result;
    use super::{FlakeRef, FlakeType};

    #[test]
    fn test_it_parses_flake_indirect() -> Result<()> {
        let url1 = "flake:nixpkgs/nixpkgs-unstable/a3a3dda3bacf61e8a39258a0ed9c924eeca8e293";
        let ref1 = super::parse(url1)?;
        assert_eq!(ref1.flake_url(), url1);
        assert_eq!(ref1.flake_type(), FlakeType::Indirect);
        assert_eq!(ref1.arg("ref"), Some("nixpkgs-unstable".to_string()));
        assert_eq!(ref1.arg("rev"), Some("a3a3dda3bacf61e8a39258a0ed9c924eeca8e293".to_string()));

        let url2 = "flake:nixpkgs/nixpkgs-unstable";
        let ref2 = super::parse(url2)?;
        assert_eq!(ref2.flake_url(), url2);
        assert_eq!(ref2.flake_type(), FlakeType::Indirect);
        assert_eq!(ref2.arg("ref"), Some("nixpkgs-unstable".to_string()));
        assert_eq!(ref2.arg("rev"), None);

        let url3 = "flake:nixpkgs";
        let ref3 = super::parse(url3)?;
        assert_eq!(ref3.flake_url(), url3);
        assert_eq!(ref3.flake_type(), FlakeType::Indirect);
        assert_eq!(ref3.arg("ref"), None);
        assert_eq!(ref3.arg("rev"), None);

        Ok(())
    }

    #[test]
    fn test_it_parses_flake_path() -> Result<()> {
        let url1 = "path:./test/path?dir=subdir";
        let ref1 = super::parse(url1)?;
        assert_eq!(ref1.flake_url(), url1);
        assert_eq!(ref1.flake_type(), FlakeType::Path);
        assert_eq!(ref1.arg("dir"), Some("subdir".to_string()));

        let url2 = "path:./test";
        let ref2 = super::parse(url2)?;
        assert_eq!(ref2.flake_url(), url2);
        assert_eq!(ref2.flake_type(), FlakeType::Path);
        assert_eq!(ref2.arg("dir"), None);
        Ok(())
    }

    #[test]
    fn test_it_parses_git_url() -> Result<()> {
        let url1 = "git+https://github.com/chadac/nixspace?rev=a3a3ddd";
        let ref1 = super::parse(url1)?;
        assert_eq!(ref1.flake_url(), url1);
        assert_eq!(ref1.flake_type(), FlakeType::Git);
        assert_eq!(ref1.git_remote_url(), Some("https://github.com/chadac/nixspace".to_string()));
        assert_eq!(ref1.arg("rev"), Some("a3a3ddd".to_string()));

        let url2 = "git+ssh://github.com/chadac/nixspace";
        let ref2 = super::parse(url2)?;
        assert_eq!(ref2.flake_url(), url2);
        assert_eq!(ref2.flake_type(), FlakeType::Git);
        assert_eq!(ref2.git_remote_url(), Some("ssh://github.com/chadac/nixspace".to_string()));
        assert_eq!(ref2.arg("rev"), None);

        let url3 = "git+file:/share/repo";
        let ref3 = super::parse(url3)?;
        assert_eq!(ref3.flake_url(), url3);
        assert_eq!(ref3.flake_type(), FlakeType::Git);
        assert_eq!(ref3.git_remote_url(), Some("file:/share/repo".to_string()));
        Ok(())
    }

    #[test]
    fn test_it_parses_mercurial_url() -> Result<()> {
        let url1 = "mc+https://github.com/chadac/nixspace?rev=a3a3ddd";
        let ref1 = super::parse(url1)?;
        assert_eq!(ref1.flake_url(), url1);
        assert_eq!(ref1.flake_type(), FlakeType::Mercurial);
        assert_eq!(ref1.arg("rev"), Some("a3a3ddd".to_string()));

        let url2 = "mc+ssh://github.com/chadac/nixspace";
        let ref2 = super::parse(url2)?;
        assert_eq!(ref2.flake_url(), url2);
        assert_eq!(ref2.flake_type(), FlakeType::Mercurial);
        assert_eq!(ref2.arg("rev"), None);

        let url3 = "mc+file:/share/repo";
        let ref3 = super::parse(url3)?;
        assert_eq!(ref3.flake_url(), url3);
        assert_eq!(ref3.flake_type(), FlakeType::Mercurial);
        Ok(())
    }

    #[test]
    fn test_it_parses_github_url() -> Result<()> {
        let url1 = "github:chadac/dotfiles/nix-config";
        let ref1 = super::parse(url1)?;
        assert_eq!(ref1.flake_url(), url1);
        assert_eq!(ref1.flake_type(), FlakeType::GitHub);
        assert_eq!(ref1.git_remote_url(), Some("https://github.com/chadac/dotfiles.git".to_string()));
        assert_eq!(ref1.arg("owner"), Some("chadac".to_string()));
        assert_eq!(ref1.arg("repo"), Some("dotfiles".to_string()));
        assert_eq!(ref1.arg("rev_or_ref"), Some("nix-config".to_string()));
        Ok(())
    }
}
