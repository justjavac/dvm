use crate::configrc::rc_get;
use crate::consts::{DENO_EXE, DVM_CACHE_PATH_PREFIX, DVM_CANARY_PATH_PREFIX, DVM_CONFIGRC_KEY_DENO_VERSION};
use crate::version::VersionArg;
use anyhow::Result;
use dirs::home_dir;
use semver::{Version, VersionReq};
use std::env;
use std::fs::write;
use std::io::{stdin, Read, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::time;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

pub fn run_with_spinner(
  message: &'static str,
  finish_message: &'static str,
  f: impl FnOnce(Box<dyn FnOnce(String) -> Result<()>>) -> Result<()>,
) -> Result<()> {
  let spinner = indicatif::ProgressBar::new_spinner().with_message(message);
  spinner.set_style(
    indicatif::ProgressStyle::default_spinner()
      .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
      .template("{spinner:.green} {msg}")
      .unwrap(),
  );
  spinner.enable_steady_tick(time::Duration::from_millis(100));
  let result = f(Box::new({
    let spinner = spinner.clone();
    move |err| {
      spinner.finish_and_clear();
      eprintln!("{}", err);
      std::process::exit(1);
    }
  }));
  spinner.finish_with_message(format!("{} in {}s", finish_message, spinner.elapsed().as_secs_f32()));

  result
}

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
  rc_get(DVM_CONFIGRC_KEY_DENO_VERSION)
    .map(|v| VersionArg::from_str(&v).unwrap())
    .unwrap_or_else(|_| VersionArg::from_str("*").unwrap())
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
