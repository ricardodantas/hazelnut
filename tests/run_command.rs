//! End-to-end tests for the one-shot `hazelnut run` command.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Two watched folders, each restricted to its own rule (the config from issue #43).
fn write_per_watch_config(root: &Path) -> PathBuf {
    let a = root.join("a");
    let b = root.join("b");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    std::fs::write(a.join("doc.pdf"), "a").unwrap();
    std::fs::write(b.join("doc.pdf"), "b").unwrap();

    let config = format!(
        r#"
[[watch]]
path = "{a}"
rules = ["Only for a"]

[[watch]]
path = "{b}"
rules = ["Only for b"]

[[rule]]
name = "Only for a"
[rule.condition]
extension = "pdf"
[rule.action]
type = "move"
destination = "{a_sorted}"

[[rule]]
name = "Only for b"
[rule.condition]
extension = "txt"
[rule.action]
type = "move"
destination = "{b_sorted}"
"#,
        a = a.display(),
        b = b.display(),
        a_sorted = root.join("a-sorted").display(),
        b_sorted = root.join("b-sorted").display(),
    );
    let config_path = root.join("config.toml");
    std::fs::write(&config_path, config).unwrap();
    config_path
}

fn hazelnut_run(config: &Path, extra_args: &[&str]) -> Output {
    let output = Command::new(env!("CARGO_BIN_EXE_hazelnut"))
        .arg("--config")
        .arg(config)
        .arg("run")
        .args(extra_args)
        .env("NO_COLOR", "1")
        .output()
        .expect("failed to run hazelnut");
    assert!(
        output.status.success(),
        "hazelnut run failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn planned_actions(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.trim_start().starts_with("[dry-run]"))
        .map(str::to_owned)
        .collect()
}

#[test]
fn run_should_only_apply_rules_listed_for_each_watch() {
    let temp = tempfile::tempdir().unwrap();
    let config = write_per_watch_config(temp.path());

    let output = hazelnut_run(&config, &[]);

    let a_doc = temp.path().join("a").join("doc.pdf");
    assert_eq!(
        planned_actions(&output),
        vec![format!(
            "  [dry-run] {} -> Move {{ destination: {:?}, create_destination: true, overwrite: false }}",
            a_doc.display(),
            temp.path().join("a-sorted")
        )]
    );
}

#[test]
fn run_with_dir_should_use_rules_of_the_watch_containing_it() {
    let temp = tempfile::tempdir().unwrap();
    let config = write_per_watch_config(temp.path());
    let b = temp.path().join("b");

    let output = hazelnut_run(&config, &["--dir", b.to_str().unwrap()]);

    assert_eq!(planned_actions(&output), Vec::<String>::new());
}

#[test]
fn run_with_dir_outside_every_watch_should_apply_all_rules() {
    let temp = tempfile::tempdir().unwrap();
    let config = write_per_watch_config(temp.path());
    let other = temp.path().join("other");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(other.join("notes.txt"), "x").unwrap();

    let output = hazelnut_run(&config, &["--dir", other.to_str().unwrap()]);

    assert_eq!(
        planned_actions(&output),
        vec![format!(
            "  [dry-run] {} -> Move {{ destination: {:?}, create_destination: true, overwrite: false }}",
            other.join("notes.txt").display(),
            temp.path().join("b-sorted")
        )]
    );
}

#[test]
fn run_apply_should_leave_files_that_no_listed_rule_matches() {
    let temp = tempfile::tempdir().unwrap();
    let config = write_per_watch_config(temp.path());

    hazelnut_run(&config, &["--apply"]);

    assert!(temp.path().join("b").join("doc.pdf").exists());
}
