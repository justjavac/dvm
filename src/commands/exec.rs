use std::process::Stdio;

use crate::{
  consts::{DVM_VERSION_LATEST, DVM_VERSION_LTS},
  meta::DvmMeta,
  utils::{best_version, deno_version_path, is_exact_version, prompt_request},
  version::{get_latest_lts_version, remote_versions, VersionArg},
};
use anyhow::Result;
use colored::Colorize;
use semver::Version;

use super::install;

pub fn exec(meta: &mut DvmMeta, version: Option<String>, args: Vec<String>) -> Result<()> {
  let version = version.unwrap_or_else(|| DVM_VERSION_LATEST.to_string());
  let v = version.clone();

  let version = if is_exact_version(&version) {
    version
  } else if version == DVM_VERSION_LTS {
    println!("Checking for latest LTS version");
    let version = get_latest_lts_version()?;
    println!("The latest LTS version is v{}", version);
    version.to_string()
  } else if meta.has_alias(&v) {
    let version_req = meta.resolve_version_req(&v);
    match version_req {
      VersionArg::Exact(v) => v.to_string(),
      VersionArg::Lts => {
        println!("Checking for latest LTS version");
        let version = get_latest_lts_version()?;
        println!("The latest LTS version is v{}", version);
        version.to_string()
      }
      VersionArg::Range(r) => {
        // Only a range needs the full list, so an exact version or `lts` runs
        // without touching the version cache.
        let versions = remote_versions()?;
        let best = best_version(versions.iter().map(AsRef::as_ref), r.clone());
        if let Some(best) = best {
          best.to_string()
        } else {
          eprintln!("No version found for {} in {:?}", r, versions);
          std::process::exit(1);
        }
      }
    }
  } else {
    eprintln!("{}", "No such alias or version found.".red());
    std::process::exit(1);
  };

  // Every branch above yields an exact version, but parse defensively rather
  // than unwrapping in case a resolver ever returns something else.
  let parsed = Version::parse(&version).map_err(|_| anyhow::anyhow!("Resolved an invalid version: {}", version))?;
  let executable_path = deno_version_path(&parsed);

  if !executable_path.exists() {
    if prompt_request(format!("deno v{} is not installed. do you want to install it?", version).as_str()) {
      install::exec(meta, true, Some(version.clone()))?;
    } else {
      eprintln!("{}", "No such version found.".red());
      std::process::exit(1);
    }
  }

  let status = std::process::Command::new(executable_path)
    .args(args)
    .stderr(Stdio::inherit())
    .stdout(Stdio::inherit())
    .stdin(Stdio::inherit())
    .spawn()?
    .wait()?;

  // `dvm exec` is a transparent wrapper around deno, so scripts and CI have to
  // see deno's own exit code. A process killed by a signal reports no code.
  std::process::exit(status.code().unwrap_or(1))
}
