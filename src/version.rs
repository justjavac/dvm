// Copyright 2022 justjavac. All rights reserved. MIT license.
use crate::consts::{
  DVM_CACHE_PATH_PREFIX, DVM_CACHE_REMOTE_PATH, REGISTRY_LATEST_CANARY_PATH, REGISTRY_LATEST_RELEASE_PATH, DVM_CHINA_MAINLAND_REGISTRY, DVM_INTERNATIONAL_REGISTRY
};
use crate::utils::{dvm_root, is_china_mainland, is_exact_version, is_semver};
use anyhow::Result;
use chrono::{DateTime, Local};
use json_minimal::Json;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use std::fmt::Formatter;
use std::fs::{read_dir, read_to_string, write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::string::String;

pub const DVM: &str = env!("CARGO_PKG_VERSION");
const CACHE_DURATION: i32 = 60 * 60 * 24;
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

pub fn remote_versions_or_cached() -> Result<Vec<String>> {
  let versions = cached_versions();
  if let Some(versions) = versions {
    return Ok(versions);
  }
  remote_versions()
}

fn cached_versions() -> Option<Vec<String>> {
  let content = read_to_string(dvm_root().join(Path::new(DVM_CACHE_REMOTE_PATH)));

  let Ok(content) = content else {
    return None;
  };
  let cached = from_str::<Cached>(content.as_str());
  let Ok(cached) = cached else {
    return None;
  };
  let cache_time: Result<DateTime<Local>, _> = DateTime::from_str(cached.time.as_str());
  let Ok(cache_time) = cache_time else {
    return None;
  };
  let expired = (Local::now().timestamp() - cache_time.timestamp()) > CACHE_DURATION as i64;
  if expired || cached.versions.len() == 0 {
    return None;
  }
  Some(cached.versions)
}

fn cache_remote_versions(versions: &Vec<String>) {
  let cached = Cached {
    versions: versions.clone(),
    time: Local::now().to_string(),
  };
  let _ = write(
    dvm_root().join(Path::new(DVM_CACHE_REMOTE_PATH)),
    serde_json::to_string(&cached).unwrap(),
  );
}

pub fn remote_versions() -> Result<Vec<String>> {
  if is_china_mainland() {
    let response = tinyget::get(DVM_CHINA_MAINLAND_REGISTRY).send()?;
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
    cache_remote_versions(&result);
    return Ok(result);
  }

  let response = tinyget::get(DVM_INTERNATIONAL_REGISTRY)
    .with_header("User-Agent", "tinyget") // http://developer.github.com/v3/#user-agent-required
    .send()?;
  let body = response.as_str()?;
  let json = Json::parse(body.as_bytes()).unwrap();
  let mut result: Vec<String> = Vec::new();

  if let Json::ARRAY(list) = json {
    for item in &list {
      if let Json::OBJECT { name: _, value } = item.get("name").unwrap() {
        if let Json::STRING(val) = value.unbox() {
          result.push(val.replace('v', "").to_string());
        }
      }
    }
  }
  cache_remote_versions(&result);
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
