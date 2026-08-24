use crate::consts::DVM_CACHE_PATH_PREFIX;
use crate::utils::{deno_version_path, dvm_root};
use crate::version::current_version;
use anyhow::Result;
use semver::Version;
use std::fs;
use std::process::exit;

pub fn exec(version: Option<String>) -> Result<()> {
  let Some(version) = version else {
    eprintln!("Please specify the version to uninstall, for example `dvm uninstall 1.46.3`.");
    exit(1)
  };

  let target_version = match Version::parse(&version) {
    Ok(ver) => ver,
    Err(_) => {
      eprintln!("Invalid semver: {}", version);
      exit(1)
    }
  };

  let target_exe_path = deno_version_path(&target_version);

  if !target_exe_path.exists() {
    eprintln!("deno v{} is not installed.", target_version);
    exit(1)
  }

  // No `deno` on PATH simply means no version is in use, which is not an error.
  let target = target_version.to_string();
  if current_version().is_some_and(|current| current == target) {
    println!("Failed: deno v{} is in use.", target_version);
    exit(1);
  }

  let version_dir = dvm_root().join(format!("{}/{}", DVM_CACHE_PATH_PREFIX, target_version));

  fs::remove_dir_all(version_dir)?;
  println!("deno v{} removed.", target_version);

  Ok(())
}
