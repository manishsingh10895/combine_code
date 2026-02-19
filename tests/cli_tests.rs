use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
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

    let mut cmd = cargo_bin_cmd!("combine_code");
    cmd.arg(dir.path())
        .arg("--exts")
        .arg("rs")
        .arg("--ignore-dirs")
        .arg("target")
        .arg("--output")
        .arg(dir.path().join("merged.txt"));

    cmd.assert().success();

    let content = fs::read_to_string(dir.path().join("merged.txt"))?;

    assert!(content.contains("good.rs"));
    assert!(!content.contains("bad.rs")); // Should be absent due to ignore-dirs

    Ok(())
}

#[test]
fn test_dry_run_lists_files_without_creating_output() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    fs::write(dir.path().join("a.rs"), "fn a() {}")?;
    fs::write(dir.path().join("b.rs"), "fn b() {}")?;

    let output_path = dir.path().join("merged.txt");
    let mut cmd = cargo_bin_cmd!("combine_code");
    cmd.arg(dir.path())
        .arg("--exts")
        .arg("rs")
        .arg("--dry-run")
        .arg("--output")
        .arg(&output_path);

    cmd.assert()
        .success()
        .stdout(contains("a.rs").and(contains("b.rs")));

    assert!(!output_path.exists());
    Ok(())
}

#[test]
fn test_stdout_writes_merged_content() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    fs::write(dir.path().join("main.rs"), "fn main() {}")?;

    let mut cmd = cargo_bin_cmd!("combine_code");
    cmd.arg(dir.path()).arg("--exts").arg("rs").arg("--stdout");

    cmd.assert()
        .success()
        .stdout(contains("# --- FILE:").and(contains("main.rs")));

    Ok(())
}

#[test]
fn test_files_are_merged_in_deterministic_order() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    fs::write(dir.path().join("z.rs"), "fn z() {}")?;
    fs::write(dir.path().join("a.rs"), "fn a() {}")?;

    let output_path = dir.path().join("merged.txt");
    let mut cmd = cargo_bin_cmd!("combine_code");
    cmd.arg(dir.path())
        .arg("--exts")
        .arg("rs")
        .arg("--output")
        .arg(&output_path);

    cmd.assert().success();

    let merged = fs::read_to_string(output_path)?;
    let a_pos = merged.find("a.rs").expect("a.rs should be present");
    let z_pos = merged.find("z.rs").expect("z.rs should be present");
    assert!(a_pos < z_pos, "a.rs should appear before z.rs");

    Ok(())
}

#[test]
fn test_include_hidden_flag_controls_hidden_files() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    fs::write(dir.path().join(".hidden.rs"), "fn hidden() {}")?;

    let without_flag = dir.path().join("without.txt");
    let mut cmd = cargo_bin_cmd!("combine_code");
    cmd.arg(dir.path())
        .arg("--exts")
        .arg("rs")
        .arg("--output")
        .arg(&without_flag);
    cmd.assert().success();
    let without_content = fs::read_to_string(&without_flag)?;
    assert!(!without_content.contains(".hidden.rs"));

    let with_flag = dir.path().join("with.txt");
    let mut cmd = cargo_bin_cmd!("combine_code");
    cmd.arg(dir.path())
        .arg("--exts")
        .arg("rs")
        .arg("--include-hidden")
        .arg("--output")
        .arg(&with_flag);
    cmd.assert().success();
    let with_content = fs::read_to_string(&with_flag)?;
    assert!(with_content.contains(".hidden.rs"));

    Ok(())
}

#[test]
fn test_exclude_glob_filters_matching_files() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    fs::write(dir.path().join("keep.rs"), "fn keep() {}")?;
    fs::write(dir.path().join("skip_me.rs"), "fn skip() {}")?;

    let output_path = dir.path().join("merged.txt");
    let mut cmd = cargo_bin_cmd!("combine_code");
    cmd.arg(dir.path())
        .arg("--exts")
        .arg("rs")
        .arg("--exclude-glob")
        .arg("*skip*")
        .arg("--output")
        .arg(&output_path);
    cmd.assert().success();

    let content = fs::read_to_string(output_path)?;
    assert!(content.contains("keep.rs"));
    assert!(!content.contains("skip_me.rs"));

    Ok(())
}

#[test]
fn test_encoding_policy_skip_skips_non_utf8_files() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    fs::write(dir.path().join("good.rs"), "fn good() {}\n")?;
    fs::write(dir.path().join("bad.rs"), [0xff, 0xfe, 0xfd])?;

    let output_path = dir.path().join("merged.txt");
    let mut cmd = cargo_bin_cmd!("combine_code");
    cmd.arg(dir.path())
        .arg("--exts")
        .arg("rs")
        .arg("--encoding-policy")
        .arg("skip")
        .arg("--output")
        .arg(&output_path);
    cmd.assert().success();

    let content = fs::read_to_string(output_path)?;
    assert!(content.contains("good.rs"));
    assert!(!content.contains("bad.rs"));

    Ok(())
}

#[test]
fn test_encoding_policy_strict_fails_on_non_utf8_files() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    fs::write(dir.path().join("bad.rs"), [0xff, 0xfe, 0xfd])?;

    let mut cmd = cargo_bin_cmd!("combine_code");
    cmd.arg(dir.path())
        .arg("--exts")
        .arg("rs")
        .arg("--encoding-policy")
        .arg("strict")
        .arg("--output")
        .arg(dir.path().join("merged.txt"));
    cmd.assert().failure();

    Ok(())
}

#[test]
fn test_encoding_policy_lossy_includes_non_utf8_files() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    fs::write(dir.path().join("bad.rs"), [0x66, 0x6f, 0x80])?;

    let output_path = dir.path().join("merged.txt");
    let mut cmd = cargo_bin_cmd!("combine_code");
    cmd.arg(dir.path())
        .arg("--exts")
        .arg("rs")
        .arg("--encoding-policy")
        .arg("lossy")
        .arg("--output")
        .arg(&output_path);
    cmd.assert().success();

    let content = fs::read_to_string(output_path)?;
    assert!(content.contains("bad.rs"));
    assert!(content.contains("fo"));

    Ok(())
}
