// Copyright 2020 justjavac. All rights reserved. MIT license.
use semver_parser::version::{parse as semver_parse, Version};
use tempfile::TempDir;

use std::env;
use std::fs;
use std::path::PathBuf;

pub fn get_dvm_root() -> PathBuf {
  match env::var_os("DVM_DIR").map(PathBuf::from) {
    Some(dvm_dir) => dvm_dir,
    None => {
      // Note: on Windows, the $HOME environment variable may be set by users or by
      // third party software, but it is non-standard and should not be relied upon.
      let home_env_var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
      let mut home_path = match env::var_os(home_env_var).map(PathBuf::from) {
        Some(home_path) => home_path,
        None => {
          // Use temp dir
          TempDir::new().unwrap().into_path()
        }
      };
      home_path.push(".dvm");
      home_path
    }
  }
}

pub fn get_exe_path(version: &Version) -> PathBuf {
  let dvm_dir = get_dvm_root().join(format!("{}", version));
  let exe_ext = if cfg!(windows) { "exe" } else { "" };
  dvm_dir.join("deno").with_extension(exe_ext)
}

pub fn is_semver(version: &str) -> bool {
  semver_parse(version).is_ok()
}
