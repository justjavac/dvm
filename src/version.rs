// Copyright 2022 justjavac. All rights reserved. MIT license.
use crate::configrc::rc_get_with_fix;
use crate::consts::{
  DVM_CACHE_PATH_PREFIX, DVM_CACHE_REMOTE_PATH, DVM_CONFIGRC_KEY_REGISTRY_VERSION, DVM_VERSION_LTS,
  REGISTRY_LATEST_CANARY_PATH,
};
use crate::utils::{deno_bin_path, dvm_root, is_exact_version, is_semver, run_with_spinner};
use anyhow::Result;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use std::fs::read_dir;
use std::io::Write;
use std::path::{Path, PathBuf};
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
  Lts,
}

impl std::fmt::Display for VersionArg {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      VersionArg::Exact(version) => f.write_str(version.to_string().as_str()),
      VersionArg::Range(version) => f.write_str(version.to_string().as_str()),
      VersionArg::Lts => f.write_str(DVM_VERSION_LTS),
    }
  }
}

impl FromStr for VersionArg {
  type Err = anyhow::Error;

  fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
    let s = s.trim();
    if s == DVM_VERSION_LTS {
      Ok(VersionArg::Lts)
    } else if is_exact_version(s) {
      Version::parse(s)
        .map(VersionArg::Exact)
        .map_err(|e| anyhow::anyhow!("Invalid semver version '{}': {}", s, e))
    } else {
      VersionReq::parse(s)
        .map(VersionArg::Range)
        .map_err(|e| anyhow::anyhow!("Invalid semver range '{}': {}", s, e))
    }
  }
}

pub fn current_version() -> Option<String> {
  let output = Command::new("deno").arg("-V").stderr(Stdio::inherit()).output().ok()?;
  if !output.status.success() {
    return None;
  }
  let stdout = String::from_utf8(output.stdout).ok()?;
  // `deno -V` prints `deno x.y.z`; return the version, or None if the format
  // is not what we expect rather than slicing into the middle of a char.
  stdout.trim().strip_prefix("deno ").map(|version| version.to_string())
}

/// Get the version of deno that dvm has activated (from dvm's bin directory).
/// Unlike `current_version()`, this does not depend on PATH, so it correctly
/// reports the dvm-managed version even when another deno installation
/// shadows dvm's bin directory on PATH.
pub fn current_dvm_version() -> Option<String> {
  let bin_path = deno_bin_path();
  if !bin_path.exists() {
    return None;
  }
  let output = Command::new(&bin_path).arg("-V").stderr(Stdio::null()).output().ok()?;
  if !output.status.success() {
    return None;
  }
  let stdout = String::from_utf8(output.stdout).ok()?;
  stdout.trim().strip_prefix("deno ").map(|version| version.to_string())
}

pub fn local_versions() -> Vec<String> {
  let mut v: Vec<String> = Vec::new();

  if let Ok(entries) = read_dir(dvm_root().join(Path::new(DVM_CACHE_PATH_PREFIX))) {
    for entry in entries.flatten() {
      if let Ok(file_type) = entry.file_type() {
        if file_type.is_dir() {
          let Ok(file_name) = entry.file_name().into_string() else {
            continue;
          };
          if is_semver(&file_name) {
            v.push(file_name);
          }
        }
      }
    }
  }

  v
}

#[inline]
pub fn cached_remote_versions_location() -> PathBuf {
  dvm_root().join(Path::new(DVM_CACHE_REMOTE_PATH))
}

pub fn cache_remote_versions() -> Result<()> {
  run_with_spinner(
    "fetching remote versions...".to_string(),
    "updated remote versions".to_string(),
    || {
      let cached_remote_versions_location = cached_remote_versions_location();

      let remote_versions_url = rc_get_with_fix(DVM_CONFIGRC_KEY_REGISTRY_VERSION)?;
      let remote_versions = tinyget::get(remote_versions_url).send()?.as_str()?.to_owned();
      crate::utils::atomic_write(cached_remote_versions_location, remote_versions.as_bytes())
        .map_err(|e| anyhow::anyhow!(e))
    },
  )
}

/// use cached remote versions if exists, otherwise ask user to fetch remote versions
pub fn remote_versions() -> Result<Vec<String>> {
  if !is_versions_cache_exists() {
    println!("It seems that you have not updated the remote version cache, please run `dvm update` first.");
    print!("Do you want to update the remote version cache now? [Y/n]");
    let _ = std::io::stdout().lock().flush();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    if input.trim().to_lowercase() == "y" || input.trim().is_empty() {
      cache_remote_versions()?;
    } else {
      anyhow::bail!("Please run `dvm update` to update the remote version cache.");
    }
  }

  let cached_remote_versions_location = cached_remote_versions_location();
  let cached_content = std::fs::read_to_string(cached_remote_versions_location)?;

  let versions = cli_versions_from_versions_json(&cached_content).map_err(|err| {
    anyhow::anyhow!(
      "Failed to parse remote versions cache: {}\nThe remote version cache is corrupted, please run `dvm update` to update the remote version cache.",
      err
    )
  })?;

  // Callers sort and match these as semver, so drop anything the registry lists
  // that is not a version instead of panicking further down the line.
  Ok(versions.into_iter().filter(|version| is_semver(version)).collect())
}

pub fn is_versions_cache_exists() -> bool {
  let remote_versions_location = cached_remote_versions_location();
  remote_versions_location.exists()
}

pub fn get_latest_remote_version(registry: &str) -> Result<Version> {
  let response = tinyget::get(registry).send()?;
  if response.status_code >= 400 {
    anyhow::bail!("Failed to fetch Deno versions: {}", response.status_code);
  }
  latest_version_from_versions_json(response.as_str()?)
}

pub fn get_latest_lts_version() -> Result<Version> {
  // Use the same versions.json endpoint as the version list — this respects
  // the user's configured registry mirror and avoids fragile GitHub HTML
  // scraping.  The latest stable Deno release IS the LTS release.
  let registry_url = rc_get_with_fix(DVM_CONFIGRC_KEY_REGISTRY_VERSION)?;
  let response = tinyget::get(&registry_url)
    .with_header("User-Agent", "dvm")
    .send()?;
  if response.status_code >= 400 {
    anyhow::bail!("Failed to fetch Deno versions: {}", response.status_code);
  }
  latest_version_from_versions_json(response.as_str()?)
}

pub fn get_latest_canary(registry: &str) -> Result<String> {
  let response = tinyget::get(format!("{}{}", registry, REGISTRY_LATEST_CANARY_PATH)).send()?;
  if response.status_code >= 400 {
    anyhow::bail!("Failed to fetch the latest canary hash: {}", response.status_code);
  }

  let body = response.as_str()?;
  Ok(body.trim().trim_start_matches('v').to_string())
}

pub fn version_req_parse(version: &str) -> Result<VersionReq> {
  VersionReq::parse(version).map_err(|err| anyhow::anyhow!("`{}` is not a valid semver range: {}", version, err))
}

pub fn find_max_matching_version<'a, I>(version_req_str: &str, iterable: I) -> Result<Option<Version>>
where
  I: IntoIterator<Item = &'a str>,
{
  let version_req = version_req_parse(version_req_str)?;
  Ok(
    iterable
      .into_iter()
      .filter_map(|s| Version::parse(s).ok())
      .filter(|s| version_req.matches(s))
      .max(),
  )
}

fn latest_version_from_versions_json(content: &str) -> Result<Version> {
  let versions = cli_versions_from_versions_json(content)?;
  versions
    .iter()
    .filter_map(|s| Version::parse(s).ok())
    .filter(|version| version.pre.is_empty())
    .max()
    .ok_or_else(|| anyhow::anyhow!("No stable Deno versions found"))
}

fn cli_versions_from_versions_json(content: &str) -> Result<Vec<String>> {
  let json: serde_json::Value = serde_json::from_str(content)?;
  let Some(cli_versions) = json.get("cli").and_then(|value| value.as_array()) else {
    anyhow::bail!("The remote version list is missing cli versions");
  };

  Ok(
    cli_versions
      .iter()
      .filter_map(|value| value.as_str())
      .map(|version| version.trim_start_matches('v').to_string())
      .collect(),
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn latest_remote_version_uses_highest_stable_cli_version() {
    let content = r#"{
      "cli": ["v2.1.11", "v2.2.8", "v3.0.0-rc.1", "v2.2.7"]
    }"#;

    assert_eq!(
      latest_version_from_versions_json(content).unwrap(),
      Version::parse("2.2.8").unwrap()
    );
  }

  #[test]
  fn cli_versions_strip_only_the_leading_v() {
    // A bare `replace('v', "")` used to corrupt versions whose pre-release
    // contained a `v`, e.g. `preview`.
    let content = r#"{ "cli": ["v2.1.0", "1.0.0-preview.1"] }"#;
    assert_eq!(
      cli_versions_from_versions_json(content).unwrap(),
      vec!["2.1.0".to_string(), "1.0.0-preview.1".to_string()]
    );
  }

  #[test]
  fn version_arg_trims_input() {
    assert_eq!(
      VersionArg::from_str(" 1.2.3 \n").unwrap(),
      VersionArg::Exact(Version::parse("1.2.3").unwrap())
    );
    assert_eq!(VersionArg::from_str(" lts \n").unwrap(), VersionArg::Lts);
  }
}
