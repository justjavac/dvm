use crate::configrc::rc_get;
use crate::consts::{DVM_CACHE_PATH_PREFIX, DVM_CONFIGRC_KEY_DENO_VERSION, DVM_VERSION_CANARY, DVM_VERSION_SYSTEM};
use crate::meta::DvmMeta;
use crate::utils::{deno_version_path, dvm_root, is_exact_version};
use anyhow::Result;
use semver::Version;
use std::fs;

pub fn exec(meta: &mut DvmMeta, version: Option<String>) -> Result<()> {
  let Some(version) = version else {
    anyhow::bail!("Please specify the version to uninstall, for example `dvm uninstall 1.46.3`.");
  };

  let target_version = Version::parse(&version).map_err(|_| anyhow::anyhow!("Invalid semver: {}", version))?;

  let target_exe_path = deno_version_path(&target_version);

  if !target_exe_path.exists() {
    anyhow::bail!("deno v{} is not installed.", target_version);
  }

  // Check if this version is the currently active one managed by dvm.
  // We check against the version that `dvm use` set as active (stored in
  // the dvm rc file), not PATH resolution.  PATH-based detection is
  // unreliable because a system-installed deno can shadow dvm's deno
  // symlink, making the check wrongly think the version isn't in use.
  let target = target_version.to_string();
  if is_active_version(meta, &target) {
    anyhow::bail!("Failed: deno v{} is currently in use by dvm.", target_version);
  }

  let version_dir = dvm_root().join(format!("{}/{}", DVM_CACHE_PATH_PREFIX, target_version));

  fs::remove_dir_all(&version_dir)?;

  // Clean up any metadata mappings pointing to this version so the metadata
  // file stays in sync with what's actually on disk.
  let _ = meta.remove_mappings_for_version(&target)?;

  println!("deno v{} removed.", target_version);

  Ok(())
}

/// Check if `target_version` is the version currently activated by `dvm use`.
///
/// This resolves the active version from dvm's configuration and metadata
/// rather than from PATH resolution, which can be wrong when a system deno
/// shadows dvm's deno symlink.
fn is_active_version(meta: &DvmMeta, target_version: &str) -> bool {
  let active_raw = match rc_get(DVM_CONFIGRC_KEY_DENO_VERSION) {
    Ok(v) => v,
    Err(_) => return false,
  };

  // Canary and system are not numbered versions managed by uninstall.
  if active_raw == DVM_VERSION_CANARY || active_raw == DVM_VERSION_SYSTEM {
    return false;
  }

  // If the active version is an exact semver, compare directly.
  if is_exact_version(&active_raw) {
    return active_raw == target_version;
  }

  // Otherwise it's a semver range or alias.  Look up its resolved mapping
  // in dvm metadata to find the exact version currently in use.
  if let Some(resolved) = meta.get_version_mapping(&active_raw) {
    return resolved == target_version;
  }

  false
}
