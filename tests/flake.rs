mod flake;
mod config;

use flake::{FlakeRef, FlakeType};
use config::UpdateStrategy;


#[test]
fn test_git_https() -> Result<()> {
    let flake_ref = flake::FlakeRef {
        flake_type: FlakeType::GitHTTPS,
        url: "//github.com/chadac/project-a",
    };
    let input_spec = flake_ref.upgrade(UpdateStrategy::LATEST)?;
    assert_eq!(input_spec, InputSpec {
        flake_type: FlakeType::GitHTTPS,
        nar_hash: Some(""),
        owner: None,
        repo: None,
        dir: None,
        rev: Some(""),
        flake_ref: Some(""),
        rev_count: None,
        last_modified: Some(0),
    });
}
