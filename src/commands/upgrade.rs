use crate::{
  commands::install,
  utils::best_version,
  version::{remote_versions, VersionArg},
  DvmMeta,
};
use anyhow::Result;
use colored::Colorize;
use std::str::FromStr;

pub fn exec(meta: &mut DvmMeta, alias: Option<String>) -> Result<()> {
  let versions = remote_versions().expect("Fetching version list failed.");
  if let Some(alias) = alias {
    if alias == "canary" {
      println!("Upgrading {}", alias.bright_black());
      install::exec(true, Some(alias)).unwrap();
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
      .unwrap_or_else(|| "N/A".to_string());
    let version_req = meta.resolve_version_req(&alias);
    match version_req {
      VersionArg::Exact(v) => {
        if current == v.to_string() {
          println!("{} is already the latest version", alias);
          std::process::exit(0);
        } else {
          install::exec(true, Some(v.to_string())).expect("Install failed");
        }
      }
      VersionArg::Range(r) => {
        let version = best_version(versions.iter().map(AsRef::as_ref).collect::<Vec<&str>>().as_slice(), r).unwrap();
        install::exec(true, Some(version.to_string())).expect("Install failed");
        meta.set_version_mapping(alias, version.to_string());
      }
    }
  } else {
    for alias in meta.list_alias() {
      let current = meta
        .get_version_mapping(alias.name.as_str())
        .unwrap_or_else(|| "N/A".to_string());

      let latest = match VersionArg::from_str(alias.required.clone().as_str()).unwrap() {
        VersionArg::Exact(v) => v.to_string(),
        VersionArg::Range(v) => best_version(versions.iter().map(AsRef::as_ref).collect::<Vec<&str>>().as_slice(), v)
          .unwrap()
          .to_string(),
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
      install::exec(true, Some(latest.clone()))?;
      meta.set_version_mapping(alias.name, latest);

      println!("Upgrading {}", "canary".bright_black());
      install::exec(true, Some("canary".to_string())).unwrap();
    }

    println!("All aliases have been upgraded");
  }

  Ok(())
}
