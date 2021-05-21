// Copyright 2020 justjavac. All rights reserved. MIT license.
use anyhow::Result;
use json_minimal::Json;

use std::fs;
use std::process::{Command, Stdio};
use std::string::String;

use crate::utils::{get_dvm_root, is_semver};

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
    for entry in entries.flatten() {
      if let Ok(file_type) = entry.file_type() {
        if file_type.is_dir() {
          let file_name = entry.file_name().into_string().unwrap();
          if is_semver(&file_name) {
            v.push(file_name);
          }
        }
      }
    }
  }

  v
}

pub fn get_remote_versions() -> Result<Vec<String>> {
  let response =
    tinyget::get("https://api.github.com/repos/denoland/deno/tags")
      // http://developer.github.com/v3/#user-agent-required
      .with_header("User-Agent", "tinyget")
      .send()?;
  let body = response.as_str()?;
  let json = Json::parse(body.as_bytes()).unwrap();
  let mut result: Vec<String> = Vec::new();

  if let Json::ARRAY(list) = json {
    for item in &list {
      if let Json::OBJECT { name: _, value } = item.get("name").unwrap() {
        if let Json::STRING(val) = value.unbox() {
          result.push(val.replace("v", "").to_string());
        }
      }
    }
  }

  Ok(result)
}
