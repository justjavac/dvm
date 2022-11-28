use crate::{
  commands::install,
  consts::{DVM_VERSION_CANARY, DVM_VERSION_INVALID, DVM_VERSION_SELF},
  utils::best_version,
  version::{remote_versions, VersionArg},
  DvmMeta,
};
use anyhow::{Ok, Result};
use colored::Colorize;
use std::fs;
use std::str::FromStr;

pub fn exec(meta: &mut DvmMeta, alias: Option<String>) -> Result<()> {
  let versions = remote_versions().expect("Fetching version list failed.");
  if let Some(alias) = alias {
    if alias == DVM_VERSION_SELF {
      upgrade_self()?;
      return Ok(());
    }

    if alias == DVM_VERSION_CANARY {
      println!("Upgrading {}", alias.bright_black());
      install::exec(meta, true, Some(alias)).unwrap();
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
          install::exec(meta, true, Some(v.to_string())).expect("Install failed");
        }
      }
      VersionArg::Range(r) => {
        let version = best_version(versions.iter().map(AsRef::as_ref), r).unwrap();
        install::exec(meta, true, Some(version.to_string())).expect("Install failed");
        meta.set_version_mapping(alias, version.to_string());
      }
    }
  } else {
    for alias in meta.list_alias() {
      let current = meta
        .get_version_mapping(alias.name.as_str())
        .unwrap_or_else(|| DVM_VERSION_INVALID.to_string());

      let latest = match VersionArg::from_str(alias.required.clone().as_str()).unwrap() {
        VersionArg::Exact(v) => v.to_string(),
        VersionArg::Range(v) => best_version(versions.iter().map(AsRef::as_ref), v).unwrap().to_string(),
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

      println!("Upgrading {}", DVM_VERSION_CANARY.bright_black());
      install::exec(meta, true, Some(DVM_VERSION_CANARY.to_string())).unwrap();
    }

    println!("All aliases have been upgraded");
  }

  Ok(())
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
      cmd.status()?;
    } else {
      let url = "https://raw.githubusercontent.com/justjavac/dvm/main/install.sh";
      let script = tinyget::get(url).send()?.as_Str()?;
      let tmp = tempfile::tempdir()?;
      let tmp = tmp.path().join("install.sh");
      fs::write(tmp, script)?;
      std::process::Command::new("bash").arg(script).status()?;
    }
  }

  Ok(())
}
