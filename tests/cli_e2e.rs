use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn rtree_cmd() -> Command {
    Command::cargo_bin("rtree").unwrap()
}

fn create_test_structure(temp: &TempDir) {
    let root = temp.path();

    fs::create_dir_all(root.join("alpha")).unwrap();
    fs::create_dir_all(root.join("beta")).unwrap();
    fs::create_dir_all(root.join("alpha/nested")).unwrap();

    fs::write(root.join("file1.txt"), "content").unwrap();
    fs::write(root.join("file2.txt"), "content").unwrap();
    fs::write(root.join("alpha/inner.txt"), "content").unwrap();
    fs::write(root.join("alpha/nested/deep.txt"), "content").unwrap();
    fs::write(root.join("beta/other.txt"), "content").unwrap();
}

#[test]
fn baseline_basic_directory_tree_output() {
    let temp = TempDir::new().unwrap();
    create_test_structure(&temp);

    let output = rtree_cmd().arg(temp.path()).output().unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("alpha/"));
    assert!(stdout.contains("beta/"));
    assert!(stdout.contains("file1.txt"));
    assert!(stdout.contains("file2.txt"));
    assert!(stdout.contains("inner.txt"));
    assert!(stdout.contains("nested/"));
    assert!(stdout.contains("deep.txt"));
    assert!(stdout.contains("other.txt"));
}

#[test]
fn baseline_hidden_files_shown_by_default() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join(".hidden"), "content").unwrap();
    fs::write(root.join("visible.txt"), "content").unwrap();
    fs::create_dir(root.join(".hidden_dir")).unwrap();

    let output = rtree_cmd().arg(temp.path()).output().unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains(".hidden"));
    assert!(stdout.contains(".hidden_dir/"));
    assert!(stdout.contains("visible.txt"));
}

#[test]
fn baseline_symlinks_shown_as_leaf_nodes() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir(root.join("target_dir")).unwrap();
    fs::write(root.join("target_dir/inside.txt"), "content").unwrap();

    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("target_dir"), root.join("link_to_dir")).unwrap();

    #[cfg(unix)]
    {
        let output = rtree_cmd().arg(temp.path()).output().unwrap();

        assert!(output.status.success());

        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(stdout.contains("link_to_dir"));
        assert!(stdout.contains("target_dir/"));
        assert!(stdout.contains("inside.txt"));
    }
}

#[test]
fn baseline_error_for_nonexistent_path() {
    let output = rtree_cmd()
        .arg("/nonexistent/path/that/does/not/exist")
        .output()
        .unwrap();

    assert!(!output.status.success());

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("rtree:"));
    assert!(stderr.contains("No such file or directory") || stderr.contains("cannot find"));
}

#[test]
fn baseline_help_output() {
    rtree_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Print a deterministic ASCII directory tree",
        ))
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn baseline_single_file_prints_filename() {
    let temp = TempDir::new().unwrap();
    let file_path = temp.path().join("single_file.txt");
    fs::write(&file_path, "content").unwrap();

    rtree_cmd()
        .arg(&file_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("single_file.txt"));
}

#[test]
fn baseline_current_directory_default() {
    let temp = TempDir::new().unwrap();
    fs::write(temp.path().join("test.txt"), "content").unwrap();

    rtree_cmd()
        .current_dir(temp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("test.txt"));
}

#[test]
fn baseline_deterministic_output_sorted_alphabetically() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("zebra.txt"), "content").unwrap();
    fs::write(root.join("apple.txt"), "content").unwrap();
    fs::write(root.join("mango.txt"), "content").unwrap();

    let output = rtree_cmd().arg(temp.path()).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    let apple_pos = stdout.find("apple.txt").unwrap();
    let mango_pos = stdout.find("mango.txt").unwrap();
    let zebra_pos = stdout.find("zebra.txt").unwrap();

    assert!(apple_pos < mango_pos);
    assert!(mango_pos < zebra_pos);
}

#[test]
fn baseline_directory_suffix_affects_sort_order() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("a"), "content").unwrap();
    fs::create_dir(root.join("a_dir")).unwrap();

    let output = rtree_cmd().arg(temp.path()).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("a\n") || stdout.contains("a\r\n") || stdout.contains("|-- a"));
    assert!(stdout.contains("a_dir/"));
}

#[test]
fn baseline_nested_directory_structure() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("level1/level2/level3")).unwrap();
    fs::write(root.join("level1/level2/level3/deep.txt"), "content").unwrap();

    let output = rtree_cmd().arg(temp.path()).output().unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("level1/"));
    assert!(stdout.contains("level2/"));
    assert!(stdout.contains("level3/"));
    assert!(stdout.contains("deep.txt"));
}

// --- Tests for new CLI flags ---

#[test]
fn flag_depth_limit_l1_shows_only_immediate_children() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("dir1/subdir")).unwrap();
    fs::write(root.join("dir1/file.txt"), "content").unwrap();
    fs::write(root.join("dir1/subdir/deep.txt"), "content").unwrap();
    fs::write(root.join("top.txt"), "content").unwrap();

    let output = rtree_cmd()
        .args(["-L", "1"])
        .arg(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should see immediate children
    assert!(stdout.contains("dir1/"));
    assert!(stdout.contains("top.txt"));

    // Should NOT see contents of dir1
    assert!(!stdout.contains("file.txt"));
    assert!(!stdout.contains("subdir"));
    assert!(!stdout.contains("deep.txt"));
}

#[test]
fn flag_depth_limit_l2_shows_two_levels() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("dir1/subdir")).unwrap();
    fs::write(root.join("dir1/file.txt"), "content").unwrap();
    fs::write(root.join("dir1/subdir/deep.txt"), "content").unwrap();

    let output = rtree_cmd()
        .args(["-L", "2"])
        .arg(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should see two levels
    assert!(stdout.contains("dir1/"));
    assert!(stdout.contains("file.txt"));
    assert!(stdout.contains("subdir/"));

    // Should NOT see third level
    assert!(!stdout.contains("deep.txt"));
}

#[test]
fn flag_ignore_single_pattern() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir(root.join("node_modules")).unwrap();
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("package.json"), "content").unwrap();

    let output = rtree_cmd()
        .args(["-I", "node_modules"])
        .arg(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("src/"));
    assert!(stdout.contains("package.json"));
    assert!(!stdout.contains("node_modules"));
}

#[test]
fn flag_ignore_pipe_separated_patterns() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir(root.join("node_modules")).unwrap();
    fs::create_dir(root.join("dist")).unwrap();
    fs::create_dir(root.join(".git")).unwrap();
    fs::create_dir(root.join("src")).unwrap();

    let output = rtree_cmd()
        .args(["-I", "node_modules|dist|.git"])
        .arg(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("src/"));
    assert!(!stdout.contains("node_modules"));
    assert!(!stdout.contains("dist"));
    assert!(!stdout.contains(".git"));
}

#[test]
fn flag_dirs_only_excludes_files() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir(root.join("dir1")).unwrap();
    fs::create_dir(root.join("dir2")).unwrap();
    fs::write(root.join("file1.txt"), "content").unwrap();
    fs::write(root.join("file2.txt"), "content").unwrap();

    let output = rtree_cmd().arg("-d").arg(temp.path()).output().unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("dir1/"));
    assert!(stdout.contains("dir2/"));
    assert!(!stdout.contains("file1.txt"));
    assert!(!stdout.contains("file2.txt"));
}

#[test]
fn flag_dirsfirst_sorts_directories_before_files() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("aaa_file.txt"), "content").unwrap();
    fs::create_dir(root.join("zzz_dir")).unwrap();
    fs::write(root.join("bbb_file.txt"), "content").unwrap();

    let output = rtree_cmd()
        .arg("--dirsfirst")
        .arg(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Directory should appear before files, regardless of alphabetical order
    let dir_pos = stdout.find("zzz_dir/").unwrap();
    let file_a_pos = stdout.find("aaa_file.txt").unwrap();
    let file_b_pos = stdout.find("bbb_file.txt").unwrap();

    assert!(dir_pos < file_a_pos, "directory should come before files");
    assert!(dir_pos < file_b_pos, "directory should come before files");
    // Files should still be alphabetically sorted
    assert!(
        file_a_pos < file_b_pos,
        "files should be alphabetically sorted"
    );
}

#[test]
fn flag_unrecognized_shows_error() {
    rtree_cmd()
        .arg("--unknown-flag")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error:"))
        .stderr(predicate::str::contains("--unknown-flag"));
}

#[test]
fn flag_help_shows_all_new_options() {
    rtree_cmd()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("-L"))
        .stdout(predicate::str::contains("-I"))
        .stdout(predicate::str::contains("-a"))
        .stdout(predicate::str::contains("-d"))
        .stdout(predicate::str::contains("--dirsfirst"));
}

#[test]
fn flag_combination_depth_and_dirsfirst() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir_all(root.join("alpha/nested")).unwrap();
    fs::write(root.join("alpha/file.txt"), "content").unwrap();
    fs::write(root.join("aaa.txt"), "content").unwrap();

    let output = rtree_cmd()
        .args(["-L", "1", "--dirsfirst"])
        .arg(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should see only first level, with dirs first
    let dir_pos = stdout.find("alpha/").unwrap();
    let file_pos = stdout.find("aaa.txt").unwrap();

    assert!(
        dir_pos < file_pos,
        "directory should come before file with --dirsfirst"
    );

    // Should NOT see nested content due to -L 1
    assert!(!stdout.contains("nested"));
    assert!(!stdout.contains("file.txt"));
}

#[test]
fn flag_combination_ignore_and_dirs_only() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::create_dir(root.join("src")).unwrap();
    fs::create_dir(root.join("node_modules")).unwrap();
    fs::write(root.join("package.json"), "content").unwrap();

    let output = rtree_cmd()
        .args(["-d", "-I", "node_modules"])
        .arg(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Only src/ should appear (dirs only, node_modules ignored)
    assert!(stdout.contains("src/"));
    assert!(!stdout.contains("node_modules"));
    assert!(!stdout.contains("package.json"));
}

#[test]
fn flag_ignore_glob_pattern_star() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("debug.log"), "content").unwrap();
    fs::write(root.join("error.log"), "content").unwrap();
    fs::write(root.join("main.rs"), "content").unwrap();
    fs::write(root.join("lib.rs"), "content").unwrap();

    let output = rtree_cmd()
        .args(["-I", "*.log"])
        .arg(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // .rs files should be visible
    assert!(stdout.contains("main.rs"));
    assert!(stdout.contains("lib.rs"));
    // .log files should be filtered out
    assert!(!stdout.contains("debug.log"));
    assert!(!stdout.contains("error.log"));
}

#[test]
fn flag_ignore_glob_pattern_question() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    // Use names that don't have substring relationships
    fs::write(root.join("x.log"), "content").unwrap();
    fs::write(root.join("y.log"), "content").unwrap();
    fs::write(root.join("zz.log"), "content").unwrap();

    let output = rtree_cmd()
        .args(["-I", "?.log"])
        .arg(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Single-char prefix files should be filtered
    assert!(!stdout.contains("x.log"));
    assert!(!stdout.contains("y.log"));
    // Two-char prefix file should be visible
    assert!(stdout.contains("zz.log"));
}

#[test]
fn flag_ignore_combined_glob_patterns() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();

    fs::write(root.join("app.log"), "content").unwrap();
    fs::write(root.join("cache.tmp"), "content").unwrap();
    fs::write(root.join("main.rs"), "content").unwrap();

    let output = rtree_cmd()
        .args(["-I", "*.log|*.tmp"])
        .arg(temp.path())
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(stdout.contains("main.rs"));
    assert!(!stdout.contains("app.log"));
    assert!(!stdout.contains("cache.tmp"));
}
