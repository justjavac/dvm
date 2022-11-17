// Copyright 2022 justjavac. All rights reserved. MIT license.
use crate::configrc::rc_get;
use crate::consts::{
  DVM_CACHE_PATH_PREFIX, DVM_CONFIGRC_KEY_REGISTRY_VERSION, REGISTRY_LATEST_CANARY_PATH, REGISTRY_LATEST_RELEASE_PATH,
  REGISTRY_LIST_OFFICIAL,
};
use crate::utils::{dvm_root, is_exact_version, is_semver};
use anyhow::Result;
use json_minimal::Json;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use std::fs::read_dir;
use std::path::Path;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::string::String;

pub const DVM: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Serialize, Deserialize)]
pub struct Cached {
  versions: Vec<String>,
  time: String,
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum VersionArg {
  Exact(Version),
  Range(VersionReq),
}

impl std::fmt::Display for VersionArg {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      VersionArg::Exact(version) => f.write_str(version.to_string().as_str()),
      VersionArg::Range(version) => f.write_str(version.to_string().as_str()),
    }
  }
}

impl FromStr for VersionArg {
  type Err = ();

  fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
    if is_exact_version(s) {
      Version::parse(s).map(VersionArg::Exact).map_err(|_| ())
    } else {
      VersionReq::parse(s)
        .map(VersionArg::Range)
        .or_else(|_| VersionReq::parse("*").map(VersionArg::Range).map_err(|_| ()))
    }
  }
}

pub fn current_version() -> Option<String> {
  match Command::new("deno").arg("-V").stderr(Stdio::inherit()).output() {
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

pub fn local_versions() -> Vec<String> {
  let mut v: Vec<String> = Vec::new();

  if let Ok(entries) = read_dir(dvm_root().join(Path::new(DVM_CACHE_PATH_PREFIX))) {
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

pub fn remote_versions() -> Result<Vec<String>> {
  let url = rc_get(DVM_CONFIGRC_KEY_REGISTRY_VERSION).unwrap_or_else(|_| REGISTRY_LIST_OFFICIAL.to_string());
  let response = tinyget::get(url).send()?;
  let body = response.as_str()?;
  let json = Json::parse(body.as_bytes()).unwrap();
  let mut result: Vec<String> = Vec::new();

  if let Json::OBJECT { name: _, value } = json.get("cli").unwrap() {
    if let Json::ARRAY(list) = value.unbox() {
      for item in list {
        if let Json::STRING(val) = item.unbox() {
          result.push(val.replace('v', "").to_string());
        }
      }
    }
  }
  Ok(result)
}

pub fn get_latest_version(registry: &str) -> Result<Version> {
  let response = tinyget::get(format!("{}{}", registry, REGISTRY_LATEST_RELEASE_PATH)).send()?;

  let body = response.as_str()?;
  let v = body.trim().replace('v', "");
  Ok(Version::parse(&v).unwrap())
}

pub fn get_latest_canary(registry: &str) -> Result<String> {
  let response = tinyget::get(format!("{}{}", registry, REGISTRY_LATEST_CANARY_PATH)).send()?;

  let body = response.as_str()?;
  let v = body.trim().replace('v', "");
  Ok(v)
}

pub fn version_req_parse(version: &str) -> VersionReq {
  VersionReq::parse(version).unwrap_or_else(|_| panic!("version is invalid: {}", version))
}

pub fn find_max_matching_version<'a, I>(version_req_str: &str, iterable: I) -> Result<Option<Version>>
where
  I: IntoIterator<Item = &'a str>,
{
  let version_req = version_req_parse(version_req_str);
  Ok(
    iterable
      .into_iter()
      .filter_map(|s| Version::parse(s).ok())
      .filter(|s| version_req.matches(s))
      .max(),
  )
}
