use crate::commands::install;
use crate::deno_bin_path;
use crate::meta::DvmMeta;
use crate::utils::{best_version, deno_canary_path, deno_version_path, prompt_request, update_stub};
use crate::utils::{is_exact_version, load_dvmrc};
use crate::version::remote_versions;
use crate::version::{get_latest_version, VersionArg};
use anyhow::Result;
use semver::Version;
use std::fs;
use std::path::Path;
use std::process::Command;

/// using a tag or a specific version
pub fn exec(meta: &mut DvmMeta, version: Option<String>, local: bool) -> Result<()> {
  let version_req: VersionArg;
  if let Some(ref version) = version {
    if version == &"canary".to_string() {
      let canary_path = deno_canary_path();
      if !canary_path.exists() {
        if prompt_request("deno canary is not installed. do you want to install it?") {
          install::exec(true, Some("canary".to_string())).unwrap();
          use_canary_bin_path(local).unwrap();
        } else {
          std::process::exit(1);
        }
      }

      use_canary_bin_path(local).unwrap();
      return Ok(());
    }

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
    if prompt_request(format!("deno v{} is not installed. do you want to install it?", used_version).as_str()) {
      install::exec(true, Some(used_version.to_string())).unwrap();
      let temp = version_req.to_string();
      let version = version.as_ref().unwrap_or(&temp);
      if !is_exact_version(version) {
        meta.set_version_mapping(version.clone(), used_version.to_string());
      }
    } else {
      std::process::exit(1);
    }
  }

  use_this_bin_path(
    &new_exe_path,
    &used_version,
    version.unwrap_or_else(|| "latest".to_string()),
    local,
  )?;
  update_stub(used_version.to_string().as_str());
  Ok(())
}

pub fn use_canary_bin_path(local: bool) -> Result<()> {
  let canary_dir = deno_canary_path();

  if !canary_dir.exists() {
    eprintln!("Canary dir not found, will not be used");
    std::process::exit(1);
  }

  let bin_path = deno_bin_path();
  if !bin_path.parent().unwrap().exists() {
    fs::create_dir_all(bin_path.parent().unwrap()).unwrap();
  }
  if bin_path.exists() {
    fs::remove_file(&bin_path)?;
  }
  fs::hard_link(&canary_dir, &bin_path)?;

  if local {
    println!("Writing to current folder config");
    fs::write(std::path::Path::new("./.dvmrc"), "canary")?;
  } else {
    println!("Writing to home folder config");
    fs::write(dirs::home_dir().unwrap().join(".dvmrc"), "canary")?;
  }

  println!("Now using deno canary");
  Ok(())
}

pub fn use_this_bin_path(exe_path: &Path, version: &Version, raw_version: String, local: bool) -> Result<()> {
  check_exe(exe_path, version)?;

  let bin_path = deno_bin_path();
  if !bin_path.parent().unwrap().exists() {
    fs::create_dir_all(bin_path.parent().unwrap()).unwrap();
  }
  if bin_path.exists() {
    fs::remove_file(&bin_path)?;
  }
  fs::hard_link(&exe_path, &bin_path)?;

  if local {
    println!("Writing to current folder config");
    fs::write(std::path::Path::new("./.dvmrc"), raw_version)?;
  } else {
    println!("Writing to home folder config");
    fs::write(dirs::home_dir().unwrap().join(".dvmrc"), raw_version)?;
  }
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
