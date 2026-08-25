use crate::{
  commands::install,
  consts::{DVM_VERSION_CANARY, DVM_VERSION_INVALID, DVM_VERSION_SELF},
  utils::{best_version, deno_canary_path},
  version::{get_latest_lts_version, remote_versions, VersionArg},
  DvmMeta,
};
use anyhow::{Ok, Result};
use colored::Colorize;
use std::fs;
use std::str::FromStr;

pub fn exec(meta: &mut DvmMeta, alias: Option<String>) -> Result<()> {
  if let Some(alias) = alias {
    if alias == DVM_VERSION_SELF {
      upgrade_self()?;
      return Ok(());
    }

    if alias == DVM_VERSION_CANARY {
      println!("Upgrading {}", alias.bright_black());
      install::exec(meta, true, Some(alias))?;
      println!("All aliases have been upgraded");
      return Ok(());
    }

    if !meta.has_alias(&alias) {
      eprintln!(
        "{} is not a valid semver version or tag and will not be upgraded",
        alias.bright_black()
      );
      std::process::exit(1);
    }
    println!("Upgrading alias {}", alias.bright_black());
    let current = meta
      .get_version_mapping(alias.as_str())
      .unwrap_or_else(|| DVM_VERSION_INVALID.to_string());
    let version_req = meta.resolve_version_req(&alias);
    match version_req {
      VersionArg::Exact(v) => {
        if current == v.to_string() {
          println!("{} is already the latest version", alias);
          std::process::exit(0);
        } else {
          install::exec(meta, true, Some(v.to_string()))?;
        }
      }
      VersionArg::Lts => {
        let version = get_latest_lts_version()?;
        install::exec(meta, true, Some(version.to_string()))?;
        meta.set_version_mapping(alias, version.to_string());
      }
      VersionArg::Range(r) => {
        // Only a semver range needs the full list, so it is fetched here rather
        // than up front: `dvm upgrade self` and `dvm upgrade canary` would
        // otherwise prompt for a version-cache update they never read.
        let versions = remote_versions()?;
        let version = match_version(&versions, &r)?;
        install::exec(meta, true, Some(version.to_string()))?;
        meta.set_version_mapping(alias, version.to_string());
      }
    }
  } else {
    let versions = remote_versions()?;
    for alias in meta.list_alias() {
      let current = meta
        .get_version_mapping(alias.name.as_str())
        .unwrap_or_else(|| DVM_VERSION_INVALID.to_string());

      let latest = match VersionArg::from_str(alias.required.clone().as_str()).unwrap() {
        VersionArg::Exact(v) => v.to_string(),
        VersionArg::Lts => get_latest_lts_version()?.to_string(),
        VersionArg::Range(v) => match_version(&versions, &v)?.to_string(),
      };

      if current == latest {
        continue;
      }

      println!(
        "Upgrading {} from {} to {}",
        alias.name.bright_black(),
        current.bright_red(),
        latest.clone().bright_green()
      );
      install::exec(meta, true, Some(latest.clone()))?;
      meta.set_version_mapping(alias.name, latest);
    }

    // canary is not an alias, so it lives outside the loop: upgrading it in the
    // loop body re-downloaded it once per upgraded alias, and skipped it
    // entirely when every alias was already current. Only refresh a canary the
    // user actually installed — otherwise `dvm upgrade` would pull a canary
    // build for someone who never asked for one.
    if deno_canary_path().exists() {
      println!("Upgrading {}", DVM_VERSION_CANARY.bright_black());
      install::exec(meta, true, Some(DVM_VERSION_CANARY.to_string()))?;
    }

    println!("All aliases have been upgraded");
  }

  Ok(())
}

/// Highest installed-or-available version satisfying `req`, erroring instead of
/// panicking when the registry lists nothing that matches.
fn match_version(versions: &[String], req: &semver::VersionReq) -> Result<semver::Version> {
  best_version(versions.iter().map(AsRef::as_ref), req.clone())
    .ok_or_else(|| anyhow::anyhow!("No released Deno version matches `{}`", req))
}

fn upgrade_self() -> Result<()> {
  cfg_if::cfg_if! {
    if #[cfg(windows)] {
      let url = "https://raw.githubusercontent.com/justjavac/dvm/main/install.ps1";
      let script = tinyget::get(url).send()?;
      let script = script.as_str()?;
      let tmp = tempfile::tempdir()?;
      let tmp = tmp.path().join("install.ps1");
      fs::write(&tmp, script)?;
      let mut cmd = std::process::Command::new("powershell");
      cmd.arg("-ExecutionPolicy").arg("Bypass").arg("-File").arg(tmp);
      let status = cmd.status()?;
      if !status.success() {
        anyhow::bail!("Failed to upgrade dvm");
      }
    } else {
      let url = "https://raw.githubusercontent.com/justjavac/dvm/main/install.sh";
      let script = tinyget::get(url).send()?;
      let script = script.as_str()?;
      let tmp = tempfile::tempdir()?;
      let tmp = tmp.path().join("install.sh");
      fs::write(&tmp, script)?;
      let status = std::process::Command::new("bash").arg(&tmp).status()?;
      if !status.success() {
        anyhow::bail!("Failed to upgrade dvm");
      }
    }
  }

  Ok(())
}
