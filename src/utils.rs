use crate::consts::{DENO_EXE, DVM_CACHE_PATH_PREFIX, DVM_CANARY_PATH_PREFIX};
use crate::version::VersionArg;
use anyhow::anyhow;
use dirs::home_dir;
use semver::{Version, VersionReq};
use std::env;
use std::fs::{read_to_string, write};
use std::io::{stdin, Read, Write};
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

pub fn prompt_request(prompt: &str) -> bool {
  print!("{} (Y/n)", prompt);

  std::io::stdout().flush().unwrap();
  let confirm = stdin()
    .bytes()
    .next()
    .and_then(|it| it.ok())
    .map(char::from)
    .unwrap_or_else(|| 'y');
  confirm == '\n' || confirm == '\r' || confirm.to_ascii_lowercase() == 'y'
}

pub fn check_is_deactivated() -> bool {
  let mut home = dvm_root();
  home.push(".deactivated");
  home.exists() && home.is_file()
}

pub fn now() -> u128 {
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis()
}

pub fn update_stub(verison: &str) {
  let mut home = dvm_versions();
  home.push(verison);
  if home.is_dir() {
    home.push(".dvmstub");
    write(home, now().to_string()).unwrap();
  }
}

pub fn is_exact_version(input: &str) -> bool {
  Version::parse(input).is_ok()
}

#[allow(dead_code)]
pub fn is_valid_semver_range(input: &str) -> bool {
  VersionReq::parse(input).is_ok()
}

pub fn best_version<'a, T>(choices: T, required: VersionReq) -> Option<Version>
where
  T: IntoIterator<Item = &'a str>,
{
  choices
    .into_iter()
    .filter_map(|v| {
      let version = Version::parse(v).ok()?;
      required.matches(&version).then_some(version)
    })
    .max_by(|a, b| a.partial_cmp(b).unwrap())
}

///
/// Find and load the dvmrc
/// local -> user -> default
pub fn load_dvmrc() -> VersionArg {
  let project_config = PathBuf::from(".dvmrc");
  let user_config = dvm_root();

  Path::exists(project_config.as_path())
    .then_some(project_config)
    .or_else(|| Path::exists(&user_config).then_some(user_config))
    .and_then(|found| {
      read_to_string(found)
        .map_err(|e| anyhow!(e))
        .and_then(|content| VersionArg::from_str(&content).map_err(|_| anyhow!("")))
        .ok()
    })
    .unwrap_or_else(|| VersionArg::from_str("*").unwrap())
}

pub fn dvm_root() -> PathBuf {
  env::var_os("DVM_DIR").map(PathBuf::from).unwrap_or_else(|| {
    // Note: on Windows, the $HOME environment variable may be set by users or by
    // third party software, but it is non-standard and should not be relied upon.
    home_dir()
      .map(PathBuf::from)
      .map(|it| it.join(".dvm"))
      .unwrap_or_else(|| TempDir::new().unwrap().into_path().join(".dvm"))
  })
}

pub fn dvm_versions() -> PathBuf {
  let mut home = dvm_root();
  home.push(DVM_CACHE_PATH_PREFIX);
  home
}

pub fn deno_canary_path() -> PathBuf {
  let dvm_dir = dvm_root().join(DVM_CANARY_PATH_PREFIX);
  dvm_dir.join(DENO_EXE)
}

/// CGQAQ: Put hardlink to executable to this file,
///        and prepend this folder to env when dvm activated.
pub fn deno_bin_path() -> PathBuf {
  let dvm_bin_dir = dvm_root().join("bin");
  dvm_bin_dir.join(DENO_EXE)
}

pub fn deno_version_path(version: &Version) -> PathBuf {
  let dvm_dir = dvm_root().join(format!("{}/{}", DVM_CACHE_PATH_PREFIX, version));
  dvm_dir.join(DENO_EXE)
}

#[inline]
pub fn is_semver(version: &str) -> bool {
  Version::parse(version).is_ok()
}

#[inline]
pub fn is_http_like_url(url: &str) -> bool {
  url.starts_with("http://") || url.starts_with("https://")
}


#[cfg(test)]
mod tests {
  use super::*;
  use semver::VersionReq;

  #[test]
  fn test_best_version() {
    let versions = vec![
      "0.8.5",
      "0.8.0",
      "0.9.0",
      "1.0.0",
      "1.0.0-alpha",
      "1.0.0-beta",
      "0.5.0",
      "2.0.0",
    ];
    assert_eq!(
      best_version(versions.iter().map(AsRef::as_ref), VersionReq::parse("*").unwrap()),
      Some(Version::parse("2.0.0").unwrap())
    );
    assert_eq!(
      best_version(versions.iter().map(AsRef::as_ref), VersionReq::parse("^1").unwrap()),
      Some(Version::parse("1.0.0").unwrap())
    );
    assert_eq!(
      best_version(versions.iter().map(AsRef::as_ref), VersionReq::parse("~0.8").unwrap()),
      Some(Version::parse("0.8.5").unwrap())
    );
  }
}
