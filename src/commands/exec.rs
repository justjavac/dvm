use std::process::Stdio;

use crate::{
  meta::DvmMeta,
  utils::{best_version, deno_version_path, is_exact_version, prompt_request},
  version::remote_versions,
};
use anyhow::Result;
use colored::Colorize;
use semver::Version;

use super::install;

pub fn exec(meta: &mut DvmMeta, command: &str, verison: Option<&str>) -> Result<()> {
  let versions = remote_versions().expect("Failed to get remote versions");
  let version = verison.map(|v| v.to_string()).unwrap_or_else(|| "latest".to_string());

  let version = if is_exact_version(&version) {
    version
  } else if meta.has_alias(&version) {
    let req = meta.get_alias(&version).expect("Failed to get alias");
    match req {
      crate::version::VersionArg::Exact(v) => v.to_string(),
      crate::version::VersionArg::Range(range) => best_version(
        versions.iter().map(AsRef::as_ref).collect::<Vec<&str>>().as_ref(),
        range,
      )
      .unwrap()
      .to_string(),
    }
  } else {
    eprintln!("{}", "No such alias or version found.".red());
    std::process::exit(1);
  };

  let executable_path = deno_version_path(&Version::parse(&version).unwrap());

  if !executable_path.exists() {
    if prompt_request(format!("deno v{} is not installed. do you want to install it?", version).as_str()) {
      install::exec(&meta, true, Some(version.clone()))
        .unwrap_or_else(|_| panic!("Failed to install deno {}", version));
    } else {
      eprintln!("{}", "No such version found.".red());
      std::process::exit(1);
    }
  }

  let mut cmd = std::process::Command::new(executable_path)
    .arg(command)
    .stderr(Stdio::inherit())
    .stdout(Stdio::inherit())
    .stdin(Stdio::inherit())
    .spawn()
    .unwrap();

  cmd.wait().unwrap();
  Ok(())
}
