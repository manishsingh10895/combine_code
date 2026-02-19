use assert_cmd::Command;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_ignore_dirs_flag() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    
    // Create a file in a 'target' directory that we want to ignore
    let target_dir = dir.path().join("target");
    fs::create_dir(&target_dir)?;
    fs::write(target_dir.join("bad.rs"), "fn main() {}")?;
    
    // Create a file in the root we want to keep
    fs::write(dir.path().join("good.rs"), "fn main() {}")?;

    let mut cmd = Command::cargo_bin("combine_code")?;
    cmd.arg(dir.path())
       .arg("--exts").arg("rs")
       .arg("--ignore-dirs").arg("target")
       .arg("--output").arg(dir.path().join("merged.txt"));

    cmd.assert().success();

    let content = fs::read_to_string(dir.path().join("merged.txt"))?;
    
    assert!(content.contains("good.rs"));
    assert!(!content.contains("bad.rs")); // Should be absent due to ignore-dirs

    Ok(())
}