use crate::meta::DvmMeta;
use std::io::{stdin, Read, Write};

use crate::commands::install;
use crate::deno_bin_path;
use crate::utils::{best_version, deno_version_path, update_stub};
use crate::utils::{is_exact_version, load_dvmrc};
use crate::version::remote_versions;
use crate::version::{get_latest_version, VersionArg};
use anyhow::Result;
use semver::Version;
use std::fs;
use std::path::Path;
use std::process::Command;

/// using a tag or a specific version
pub fn exec(meta: &mut DvmMeta, version: Option<String>) -> Result<()> {
  let version_req: VersionArg;
  if let Some(ref version) = version {
    if is_exact_version(version) {
      version_req = VersionArg::Exact(Version::parse(version).unwrap());
    } else if meta.has_alias(version) {
      version_req = meta.resolve_version_req(version);
    } else {
      // dvm will reject for using semver range directly now.
      eprintln!(
        "`{}` is not a valid semver version or tag and will not be used\ntype `dvm help` for more info",
        version
      );
      std::process::exit(1);
    }
  } else {
    println!("No version input detect, try to use version in .dvmrc file");
    version_req = load_dvmrc();
    println!("Using semver range: {}", version_req);
  }

  let used_version = if version_req.to_string() == "*" {
    println!("Checking for latest version");
    let version = get_latest_version(&meta.registry).expect("Get latest version failed");
    println!("The latest version is v{}", version);
    version
  } else {
    match version_req {
      VersionArg::Exact(ref v) => v.clone(),
      VersionArg::Range(ref r) => {
        println!("Fetching version list");
        let versions = remote_versions().expect("Fetching version list failed.");
        best_version(
          versions.iter().map(AsRef::as_ref).collect::<Vec<&str>>().as_slice(),
          r.clone(),
        )
        .unwrap()
      }
    }
  };

  let new_exe_path = deno_version_path(&used_version);

  if !new_exe_path.exists() {
    print!(
      "deno v{} is not installed. do you want to install it? (Y/n)",
      used_version
    );
    std::io::stdout().flush().unwrap();
    let confirm = stdin()
      .bytes()
      .next()
      .and_then(|it| it.ok())
      .map(char::from)
      .unwrap_or_else(|| 'y');
    if confirm == '\n' || confirm == '\r' || confirm.to_ascii_lowercase() == 'y' {
      install::exec(true, Some(used_version.to_string())).unwrap();
      let version = version.unwrap_or_else(|| version_req.to_string());
      if !is_exact_version(&version) {
        meta.set_version_mapping(version, used_version.to_string());
        meta.save();
      }
    } else {
      std::process::exit(1);
    }
  }

  use_this_bin_path(&new_exe_path, &used_version)?;
  update_stub(used_version.to_string().as_str());
  Ok(())
}

pub fn use_this_bin_path(exe_path: &Path, version: &Version) -> Result<()> {
  check_exe(exe_path, version)?;

  let bin_path = deno_bin_path();
  if !bin_path.parent().unwrap().exists() {
    fs::create_dir_all(bin_path.parent().unwrap()).unwrap();
  }
  if bin_path.exists() {
    fs::remove_file(&bin_path)?;
  }
  fs::hard_link(&exe_path, &bin_path)?;
  println!("Now using deno {}", version);
  Ok(())
}

fn check_exe(exe_path: &Path, expected_version: &Version) -> Result<()> {
  let output = Command::new(exe_path)
    .arg("-V")
    .stderr(std::process::Stdio::inherit())
    .output()?;
  let stdout = String::from_utf8(output.stdout)?;
  assert!(output.status.success());
  assert_eq!(stdout.trim(), format!("deno {}", expected_version));
  Ok(())
}
