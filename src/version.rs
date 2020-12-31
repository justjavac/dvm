// Copyright 2020 justjavac. All rights reserved. MIT license.
use std::fs;
use std::process::{Command, Stdio};
use std::string::String;
use semver_parser::version::{parse as semver_parse};

use crate::utils::get_dvm_root;

pub const DVM: &str = env!("CARGO_PKG_VERSION");

pub fn get_current_version() -> Option<String> {
  match Command::new("deno")
    .arg("-V")
    .stderr(Stdio::inherit())
    .output()
  {
    Ok(output) => {
      assert!(output.status.success());
      match String::from_utf8(output.stdout) {
        Ok(stdout) => Some(stdout.trim()[5..].to_string()),
        Err(_) => None,
      }
    }
    Err(_) => None,
  }
}

pub fn get_local_versions() -> Vec<String> {
  let mut v: Vec<String> = Vec::new();

  if let Ok(entries) = fs::read_dir(get_dvm_root()) {
    for entry in entries {
      if let Ok(entry) = entry {
        if let Ok(file_type) = entry.file_type() {
          if file_type.is_dir() {
            let file_name = entry.file_name().into_string().unwrap();
            if let Ok(_) = semver_parse(&file_name) {
              v.push(file_name);
            }
          }
        }
      }
    }
  }

  v
}
