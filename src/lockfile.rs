use serde::{Serialize, Deserialize};
use anyhow::{anyhow, Context, Result};
use std::collections::{HashSet, BTreeMap};
use std::path::Path;
use std::rc::Rc;

use super::cli::Nix;
use super::flake::FlakeRef;

type Nodes = BTreeMap<String, LockedRef>;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct LockFile {
    nodes: Nodes,
    root: String,
    version: i32,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum InputRef {
    Direct(String),
    Path(Vec<String>),
}

impl InputRef {
    fn rename(&mut self, orig_name: &str, new_name: &str) -> () {
        match self {
            InputRef::Direct(ref mut n) => {
                if n == orig_name {
                    *n = new_name.to_string();
                }
            },
            InputRef::Path(ref mut p) => {
                if let Some(n) = p.first() {
                    if n == orig_name {
                        p.remove(0);
                        p.insert(0, new_name.to_string());
                    };
                }
            }
        }
    }

    fn head(&self) -> String {
        match self {
            InputRef::Direct(n) => n.to_string(),
            InputRef::Path(p) => p.last().unwrap().to_string(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct LockedRef {
    #[serde(skip_serializing_if = "Option::is_none")]
    flake: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    locked: Option<InputSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    original: Option<InputSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inputs: Option<BTreeMap<String, InputRef>>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nar_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rev: Option<String>,
    #[serde(rename = "ref")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
    #[serde(rename = "revCount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rev_count: Option<i64>,
    #[serde(rename = "lastModified")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<i64>,
}

impl LockedRef {
    fn empty() -> Self {
        LockedRef {
            flake: None,
            locked: None,
            original: None,
            inputs: Some(BTreeMap::new()),
        }
    }

    fn root(nodes: &Nodes) -> Self {
        todo!()
    }

    /// Generates a new LockedRef from an inputspec
    fn from(locked: &InputSpec) -> Self {
        todo!()
    }
}

impl LockFile {
    fn merge(lockfiles: &BTreeMap<String, LockFile>) -> Result<LockFile> {
        let mut l: BTreeMap<String, LockFile> = lockfiles.clone();

        // we need to rename inputs so that when merged, stuff doesn't
        // conflict with each other
        for (name, lockfile) in &mut l {
            let mut input_map = BTreeMap::<String, String>::new();

            // start by namespacing everything
            for input_name in lockfile.nodes.keys() {
                if input_name != "root" && !lockfiles.contains_key(input_name) {
                    let new_input_name = format!("{name}_{input_name}");
                    input_map.insert(input_name.to_string(), new_input_name);
                }
            }

            // then, substitute any references to shared packages with our stuff
            let root = lockfile.nodes.get("root").unwrap();
            let empty_inputs = BTreeMap::new();
            let root_inputs = root.inputs.as_ref().unwrap_or(&empty_inputs);
            for (input_name, alias) in root_inputs {
                if lockfiles.contains_key(input_name) {
                    let orig_input_name = lockfile.resolve_input(&alias);
                    input_map.insert(orig_input_name, input_name.to_string());
                }
            }

            for (input_name, new_input_name) in input_map {
                lockfile.rename_input(&input_name, &new_input_name);
            }

            // rename our root to the input_name for later merging
            lockfile.rename_input("root", name);
        }

        let mut new_nodes: Nodes = BTreeMap::new();
        for lockfile in l.values() {
            for (input_name, node) in &lockfile.nodes {
                new_nodes.insert(input_name.to_string(), node.clone());
            }
        }

        // insert a new root node pointing to each of the original lockfiles list
        new_nodes.insert("root".to_string(), LockedRef {
            flake: None,
            locked: None,
            original: None,
            inputs:Some(BTreeMap::from_iter(
                lockfiles.keys()
                    .map(|n| (n.to_string(), InputRef::Direct(n.to_string())))
            )),
        });

        let mut lockfile = Self {
            nodes: new_nodes,
            root: "root".to_string(),
            version: 7,
        };
        println!("{:?}", lockfile.nodes.keys());
        lockfile.trim()?;

        Ok(lockfile)
    }

    fn from_nodes(nodes: Nodes) -> Self {
        let mut new_nodes: Nodes = nodes
            .into_iter()
            .filter(|(name, _)| name == "root")
            .collect();
        let root = LockedRef::root(&new_nodes);
        new_nodes.insert("root".to_string(), root);
        Self {
            nodes: new_nodes,
            root: "root".to_string(),
            version: 7
        }
    }

    /// Generate an empty lockfile
    pub fn empty() -> Self {
        Self::from_nodes(BTreeMap::new())
    }

    pub fn from_metadata(projects: BTreeMap<String, super::cli::FlakeMetadata>) -> Result<Self> {
        let lockfiles: BTreeMap<String, LockFile> = BTreeMap::from_iter(
            projects.iter().map(|(n, m)| (n.to_string(), m.locks.clone()))
        );
        let mut lockfile = Self::merge(&lockfiles)?;
        println!("{:?}", lockfile.nodes.keys());
        for (name, metadata) in projects {
            let node = lockfile.nodes.get_mut(&name).with_context(
                || anyhow!("project '{name}' was missing during merge; badly formatted lockfile?")
            )?;
            node.original = Some(metadata.original.clone());
            node.locked = Some(metadata.locked.clone());
        }
        Ok(lockfile)
    }

    pub fn read(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str::<LockFile>(&contents)?)
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        std::fs::write(path, serde_json::to_string(&self)?)?;
        Ok(())
    }

    fn root_node(self) -> Result<LockedRef> {
        self.nodes.get(&self.root).map(|n| n.clone())
            .context("lockfile is missing root node! improperly formatted?")
    }

    pub fn get_input_spec(&self, name: &str) -> Option<InputSpec> {
        self.nodes.get(name).map(|r| r.locked.clone()).flatten()
    }

    fn resolve_input(&self, input_path: &InputRef) -> String {
        match input_path {
            InputRef::Direct(i) => i.to_string(),
            InputRef::Path(p) => {
                let (head, tail) = p.split_at(1);
                let node_name = head.first().unwrap();
                self.get_input_by_path(
                    node_name,
                    &Vec::from(tail)
                )
            },
        }
    }

    fn get_input_by_path(&self, node_name: &String, path: &Vec<String>) -> String {
        if path.is_empty() {
            node_name.to_string()
        } else {
            let node = self.nodes.get(node_name).unwrap();
            let (head, tail) = path.split_at(1);
            let h = head.first().unwrap();
            self.get_input_by_path(
                &self.resolve_input(node.inputs.as_ref().unwrap().get(h).unwrap()),
                &Vec::from(tail),
            )
        }
    }

    // fn resolve_input_ref(&self, node: Option<String>, input_path: &InputRef) -> Result<String> {
    //     let node_inputs_empty = BTreeMap::new();
    //     let inputs: BTreeMap<String, String> = match node {
    //         None => self.nodes.keys().map(|k| (k.to_string(), k.to_string())).collect(),
    //         Some(ref n) => {
    //             let node_1 = self.nodes.get(n);
    //             let node = node_1.as_ref()
    //                 .with_context(|| anyhow!("could not resolve node with name '{n}'; improperly formatted lockfile?"))?;
    //             let node_inputs = node.inputs.as_ref().unwrap_or(&node_inputs_empty);
    //             node_inputs.iter().map(|(k, v)| (k.to_string(), v.head())).collect()
    //         },
    //     };
    //     match input_path {
    //         InputRef::Direct(i) => {
    //             inputs.get(i)
    //                 .map(|s| s.to_string())
    //                 .with_context(|| anyhow!("could not resolve node with name '{i}'; improperly formatted lockfile?"))
    //         },
    //         InputRef::Path(p) => {
    //             let (head, tail) = p.split_at(1);
    //             if let Some(h) = head.first() {
    //                 let (h2, t2) = Vec::from(tail).split_at(1);
    //                 let parent = self.resolve_input_ref(Some(h.to_string()), );
    //                 let rest = &InputRef::Path(Vec::from(tail));
    //             } else {
    //                 node.with_context(||
    //                     anyhow!("should be unreachable??")
    //                 )
    //             }
    //         },
    //     }
    // }

    pub fn add(&mut self, name: &str, new_input_spec: &InputSpec) -> Result<()> {
        let p = self.nodes.get_mut(name)
            .ok_or(anyhow!("could not find project '{}'", name))?;
        p.locked = Some(new_input_spec.clone());
        let root = self.nodes.get_mut(&self.root)
            .context("failed parsing lockfile: missing entry 'root' in nodes")?;
        if let Some(ref mut inputs) = root.inputs {
            inputs.insert(name.to_string(), InputRef::Direct(name.to_string()));
        }
        Ok(())
    }

    pub fn rm(&mut self, name: &str) -> Result<()> {
        self.nodes.remove(name);
        let root = self.nodes.get_mut(&self.root)
            .context("failed parsing lockfile; missing entry 'root' in nodes")?;
        if let Some(ref mut inputs) = root.inputs {
            inputs.remove(name);
        }
        Ok(())
    }

    pub fn rename_input(&mut self, input_name: &str, new_name: &str) -> () {
        if !self.nodes.contains_key(input_name) {
            return
        }
        for (_, node) in &mut self.nodes {
            if let Some(ref mut inputs) = node.inputs {
                for (_, input_ref) in inputs {
                    input_ref.rename(input_name, new_name);
                }
            }
        }
        self.nodes.insert(new_name.to_string(), self.nodes.get(input_name).unwrap().clone());
        self.nodes.retain(|n, _| n != input_name);
    }

    fn closure(&self) -> Result<HashSet<String>> {
        let mut queue = Vec::from(&[ self.root.to_string() ]);
        let mut visited = HashSet::new();
        visited.insert(self.root.to_string());

        while !queue.is_empty() {
            let node_name = queue.pop().unwrap();
            visited.insert(node_name.clone());
            let node = self.nodes.get(&node_name)
                .with_context(|| anyhow!("could not find node with name '{node_name}'; improperly formatted lockfile?"))?;
            if let Some(i) = &node.inputs {
                for input_ref in i.values() {
                    let next_input = self.resolve_input(&input_ref);
                    println!("{node_name} {next_input}");
                    if !visited.contains(&next_input) {
                        queue.push(next_input);
                    }
                }
            }
        }

        Ok(visited)
    }

    pub fn trim(&mut self) -> Result<()> {
        let keep = self.closure()?;
        let mut remove: Vec<String> = self.nodes.iter().map(|(n, _)| n.to_string()).collect();
        remove.retain(|n| !keep.contains(n));
        for node in remove {
            self.rm(&node)?;
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

    pub fn nix_path(&self) -> Result<String> {
        todo!()
    }

    pub fn build(&self) -> Result<()> {
        todo!()
    }

    pub fn inputs(&self) -> Result<String> {
        todo!()
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
