use semver_parser::version::{parse as semver_parse, Version};
use std::env;
use std::path::PathBuf;
use tempfile::TempDir;

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

pub fn deno_bin_path(version: &Version) -> PathBuf {
  let dvm_dir = dvm_root().join(format!("{}", version));
  let exe_ext = if cfg!(windows) { "exe" } else { "" };
  dvm_dir.join("deno").with_extension(exe_ext)
}

pub fn is_semver(version: &str) -> bool {
  semver_parse(version).is_ok()
}

#[cfg(not(windows))]
pub fn is_china_mainland() -> bool {
  env::var("LANG").map(|lng| lng.starts_with("zh_CN.")).unwrap_or(false)
}

#[cfg(windows)]
pub fn is_china_mainland() -> bool {
  use winapi::ctypes::c_int;
  use winapi::um::winnls::GetUserDefaultLocaleName;

  // The maximum number of characters allowed for this string is 85,
  // including a terminating null character.
  // https://docs.microsoft.com/en-us/windows/win32/intl/locale-sname
  let mut buf = [0u16; 85];
  let len = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as c_int) };

  if len <= 0 {
    return false;
  }

  String::from_utf16_lossy(&buf).starts_with("zh-CN")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_is_china_mainland() {
    assert!(is_china_mainland());
  }
}
