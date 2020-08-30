// Copyright 2020 the Dvm authors. All rights reserved. MIT license.
// use anyhow::Result;
use semver_parser::version::{parse, Version};
// use which::which;

use std::fs;
use std::process::{Command, Stdio};
use std::string::String;

use crate::utils::get_dvm_root;

pub const DVM: &str = env!("CARGO_PKG_VERSION");
pub const DENO: &str = "1.3.0";

pub fn get_current_version() -> Option<Version> {
  match Command::new("deno")
    .arg("-V")
    .stderr(Stdio::inherit())
    .output()
  {
    Ok(output) => {
      assert!(output.status.success());
      match String::from_utf8(output.stdout) {
        Ok(stdout) => Some(parse(&stdout.trim()[5..]).unwrap()),
        Err(_) => None,
      }
    }
    Err(_) => None,
  }
}

pub fn get_local_versions() -> Vec<String> {
  let mut v: Vec<String> = Vec::new();

  if let Ok(entries) = fs::read_dir(get_dvm_root().unwrap()) {
    for entry in entries {
      if let Ok(entry) = entry {
        if let Ok(file_type) = entry.file_type() {
          if file_type.is_dir() {
            v.push(entry.file_name().into_string().unwrap());
          }
        }
      }
    }
  }

  v
}
