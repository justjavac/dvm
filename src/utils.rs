// Copyright 2020 the Dvm authors. All rights reserved. MIT license.
use anyhow::Result;
use tempfile::TempDir;

use std::env;
use std::path::PathBuf;

pub fn get_dvm_root() -> Result<PathBuf> {
    // Note: on Windows, the $HOME environment variable may be set by users or by
    // third party software, but it is non-standard and should not be relied upon.
    let home_env_var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    let mut home_path = match env::var_os(home_env_var).map(PathBuf::from) {
      Some(home_path) => home_path,
      None => {
        // Use temp dir
        TempDir::new()?.into_path()
      }
    };
  
    home_path.push(".dvm");
    Ok(home_path)
  }
  