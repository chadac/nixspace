use std::path::{Path, PathBuf};

pub fn find_root<P: AsRef<Path> + ?Sized>(name: &str, wd: &P) -> Option<PathBuf> {
    let mut cwd: PathBuf = PathBuf::new();
    cwd.push(wd);
    loop {
        let path = cwd.as_path().join(name);
        if path.exists() {
            return Some(cwd.as_path().into());
        }
        if !cwd.pop() {
            break
        }
    };
    None
}
