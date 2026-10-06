use crate::consts::DVM_CACHE_PATH_PREFIX;
use crate::meta::DvmMeta;
use crate::utils::{deno_version_path, dvm_root};
use crate::version::current_version;
use anyhow::Result;
use semver::Version;
use std::fs;

pub fn exec(meta: &mut DvmMeta, version: Option<String>) -> Result<()> {
  let Some(version) = version else {
    anyhow::bail!("Please specify the version to uninstall, for example `dvm uninstall 1.46.3`.");
  };

  let target_version = Version::parse(&version)
    .map_err(|_| anyhow::anyhow!("Invalid semver: {}", version))?;

  let target_exe_path = deno_version_path(&target_version);

  if !target_exe_path.exists() {
    anyhow::bail!("deno v{} is not installed.", target_version);
  }

  // No `deno` on PATH simply means no version is in use, which is not an error.
  let target = target_version.to_string();
  if current_version().is_some_and(|current| current == target) {
    anyhow::bail!("Failed: deno v{} is in use.", target_version);
  }

  let version_dir = dvm_root().join(format!("{}/{}", DVM_CACHE_PATH_PREFIX, target_version));

  fs::remove_dir_all(&version_dir)?;

  // Clean up any metadata mappings pointing to this version so the metadata
  // file stays in sync with what's actually on disk.
  let _ = meta.remove_mappings_for_version(&target)?;

  println!("deno v{} removed.", target_version);

  Ok(())
}
