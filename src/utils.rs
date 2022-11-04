use cfg_if::cfg_if;

use crate::consts::{DVM_CACHE_PATH_PREFIX, DVM_CANARY_PATH_PREFIX};
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
  let mut home = dvm_root();
  home.push("versions");
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
  let project_config = Path::new(".dvmrc");
  let user_config = home_dir().unwrap().join(".dvmrc");

  let mut found_config: Option<&Path> = None;
  if Path::exists(project_config) {
    found_config = Some(project_config)
  } else if Path::exists(user_config.as_path()) {
    found_config = Some(user_config.as_path())
  }

  if let Some(found) = found_config {
    let result = read_to_string(found)
      .map_err(|e| anyhow!(e))
      .and_then(|content| VersionArg::from_str(&content).map_err(|_| anyhow!("")));
    if let Ok(req) = result {
      return req;
    }
  }

  VersionArg::from_str("*").unwrap()
}

pub fn dvm_root() -> PathBuf {
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

pub fn deno_canary_path() -> PathBuf {
  let dvm_dir = dvm_root().join(DVM_CANARY_PATH_PREFIX);
  let exe_ext = if cfg!(windows) { "exe" } else { "" };
  dvm_dir.join("deno").with_extension(exe_ext)
}

/// CGQAQ: Put hardlink to executable to this file,
///        and prepend this folder to env when dvm activated.
pub fn deno_bin_path() -> PathBuf {
  let dvm_bin_dir = dvm_root().join("bin");
  let exe_ext = if cfg!(windows) { "exe" } else { "" };
  dvm_bin_dir.join("deno").with_extension(exe_ext)
}

pub fn deno_version_path(version: &Version) -> PathBuf {
  let dvm_dir = dvm_root().join(format!("{}/{}", DVM_CACHE_PATH_PREFIX, version));
  let exe_ext = if cfg!(windows) { "exe" } else { "" };
  dvm_dir.join("deno").with_extension(exe_ext)
}

#[inline]
pub fn is_semver(version: &str) -> bool {
  Version::parse(version).is_ok()
}

cfg_if! {
  if #[cfg(windows)] {
    pub fn is_china_mainland() -> bool {
      use winapi::ctypes::c_int;
      use winapi::um::winnls::GetUserDefaultLocaleName;

      // The maximum number of characters allowed for this string is 85,
      // including a terminating null character.
      // https://docs.microsoft.com/en-us/windows/win32/intl/locale-sname
      let mut buf = [0u16; 85];
      // SAFETY: Call `winapi` raw binding to win32 api.
      let len = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as c_int) };

      if len <= 0 {
        return false;
      }

      String::from_utf16_lossy(&buf).starts_with("zh-CN")
    }
  } else {
    pub fn is_china_mainland() -> bool {
      env::var("LANG").map(|lng| lng.starts_with("zh_CN.")).unwrap_or(false)
    }
  }
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
