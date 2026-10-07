//! Basic integration tests for dvm.
//!
//! These tests invoke the compiled dvm binary as a subprocess to verify
//! end-to-end behavior.  They are designed to be fast and do not require
//! network access.

use std::process::Command;
use tempfile::TempDir;

/// Returns the path to the compiled dvm binary.
fn dvm_bin() -> std::path::PathBuf {
  // Cargo sets this env var automatically for integration tests.
  std::path::PathBuf::from(env!("CARGO_BIN_EXE_dvm"))
}

/// Creates a temporary directory and returns a Command pre-configured
/// with DVM_DIR pointing at it.
///
/// The TempDir is returned alongside the Command because the directory
/// is deleted when the TempDir is dropped — the caller must keep it
/// alive for the duration of the test.
fn dvm_command_with_temp_dir() -> (Command, TempDir) {
  let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
  let mut cmd = Command::new(dvm_bin());
  cmd.env("DVM_DIR", temp_dir.path());
  (cmd, temp_dir)
}

#[test]
fn version_output_matches_cargo_version() {
  let output = Command::new(dvm_bin())
    .arg("--version")
    .output()
    .expect("failed to run dvm --version");

  assert!(output.status.success(), "dvm --version should exit successfully");

  let stdout = String::from_utf8_lossy(&output.stdout);
  let expected_version = format!("dvm {}", env!("CARGO_PKG_VERSION"));
  assert!(
    stdout.trim().contains(&expected_version),
    "version output should contain '{}', got: '{}'",
    expected_version,
    stdout.trim()
  );
}

#[test]
fn help_exits_successfully() {
  let output = Command::new(dvm_bin())
    .arg("--help")
    .output()
    .expect("failed to run dvm --help");

  assert!(output.status.success(), "dvm --help should exit with status 0");

  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(!stdout.is_empty(), "dvm --help should print help text to stdout");
}

#[test]
fn list_exits_successfully_with_empty_list() {
  let (mut cmd, _temp_dir) = dvm_command_with_temp_dir();
  let output = cmd.arg("list").output().expect("failed to run dvm list");

  assert!(
    output.status.success(),
    "dvm list should exit successfully (empty list is fine)\nstderr: {}",
    String::from_utf8_lossy(&output.stderr)
  );
}

#[test]
fn info_exits_successfully() {
  let (mut cmd, _temp_dir) = dvm_command_with_temp_dir();
  let output = cmd.arg("info").output().expect("failed to run dvm info");

  assert!(
    output.status.success(),
    "dvm info should exit successfully\nstderr: {}",
    String::from_utf8_lossy(&output.stderr)
  );

  let stdout = String::from_utf8_lossy(&output.stdout);
  assert!(!stdout.is_empty(), "dvm info should print info to stdout");
}
