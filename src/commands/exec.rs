use std::process::Stdio;

use crate::{
  consts::DVM_VERSION_LATEST,
  meta::DvmMeta,
  utils::{best_version, deno_version_path, is_exact_version, prompt_request},
  version::{remote_versions, VersionArg},
};
use anyhow::Result;
use colored::Colorize;
use semver::Version;

use super::install;

pub fn exec(meta: &mut DvmMeta, version: Option<String>, args: Vec<String>) -> Result<()> {
  let versions = remote_versions().expect("Failed to get remote versions");
  let version = version.unwrap_or_else(|| DVM_VERSION_LATEST.to_string());
  let v = version.clone();

  let Some(version) = is_exact_version(&version).then_some(version).or_else(|| {
        meta.has_alias(&v).then(|| {
          let version_req = meta.resolve_version_req(&v);
          match version_req {
            VersionArg::Exact(v) => v.to_string(),
            VersionArg::Range(r) => best_version(versions.iter().map(AsRef::as_ref), r).unwrap().to_string(),
          }
        })
    }
  ) else {
    eprintln!("{}", "No such alias or version found.".red());
    std::process::exit(1);
  };

  let executable_path = deno_version_path(&Version::parse(&version).unwrap());

  if !executable_path.exists() {
    if prompt_request(format!("deno v{} is not installed. do you want to install it?", version).as_str()) {
      install::exec(meta, true, Some(version.clone())).unwrap_or_else(|_| panic!("Failed to install deno {}", version));
    } else {
      eprintln!("{}", "No such version found.".red());
      std::process::exit(1);
    }
  }

  let mut cmd = std::process::Command::new(executable_path)
    .args(args)
    .stderr(Stdio::inherit())
    .stdout(Stdio::inherit())
    .stdin(Stdio::inherit())
    .spawn()
    .unwrap();

  cmd.wait().unwrap();
  Ok(())
}
