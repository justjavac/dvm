use crate::consts::DVM_CACHE_PATH_PREFIX;
use crate::utils::{deno_version_path, dvm_root};
use crate::version::current_version;
use anyhow::Result;
use semver::Version;
use std::fs;
use std::process::exit;

pub fn exec(version: Option<String>) -> Result<()> {
  let target_version = match version {
    Some(target_version) => match Version::parse(&target_version) {
      Ok(ver) => ver,
      Err(_) => {
        eprintln!("Invalid semver");
        exit(1)
      }
    },
    None => unimplemented!(),
  };
  let target_exe_path = deno_version_path(&target_version);

  println!("{}", target_exe_path.display());

  if !target_exe_path.exists() {
    eprintln!("deno v{} is not installed.", target_version);
    exit(1)
  }

  let current_version = current_version().unwrap();

  if current_version == target_version.to_string() {
    println!("Failed: deno v{} is in use.", target_version);
    exit(1);
  }

  let version_dir = dvm_root().join(format!("{}/{}", DVM_CACHE_PATH_PREFIX, target_version));

  fs::remove_dir_all(&version_dir).unwrap();
  println!("deno v{} removed.", target_version);

  Ok(())
}
