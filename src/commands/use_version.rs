use crate::meta::DvmMeta;
use std::io::{Read, stdin, Write};

use crate::utils::load_dvmrc;
use crate::utils::{best_version, deno_bin_path};
use crate::version::get_latest_version;
use crate::version::remote_versions;
use crate::commands::install;
use anyhow::Result;
use semver::{Version, VersionReq};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;

pub fn exec(meta: &mut DvmMeta, version: Option<String>) -> Result<()> {
  let version_req: VersionReq;
  if let Some(version) = version {
    version_req = meta.resolve_version_req(&version)
  } else {
    println!("No version input detect, try to use version in .dvmrc file");
    version_req = load_dvmrc();
    println!("Using {}", version_req.to_string());
  }

  let used_version = if version_req.to_string() == "*" {
    println!("Checking for latest version");
    let version = get_latest_version(&meta.registry).expect("Get latest version failed");
    println!("The latest version is v{}", version.to_string());
    version
  } else {
    println!("Fetching version list");
    let versions = remote_versions().expect("Fetching version list failed.");
    best_version(&versions.iter().map(AsRef::as_ref).collect(), version_req.clone()).unwrap()
  };

  let new_exe_path = deno_bin_path(&used_version);

  if !new_exe_path.exists() {
    print!("deno v{} is not installed. do you want to install it? (Y/n)", used_version);
    std::io::stdout().flush().unwrap();
    let confirm = stdin().bytes().next().and_then(|it| it.ok()).map(char::from).unwrap();
    if confirm == '\n' || confirm.to_ascii_lowercase() == 'y' {
      install::exec(true, Some(used_version.to_string())).unwrap();
      meta.set_version_mapping(version_req.to_string(), used_version.to_string());
      meta.save();
    } else {
      std::process::exit(1);
    }
  }

  use_this_bin_path(&new_exe_path, &used_version)?;
  Ok(())
}

pub fn use_this_bin_path(exe_path: &Path, version: &Version) -> Result<()> {
  check_exe(exe_path, version)?;

  let old_exe_path = match which("deno") {
    Ok(old_exe_path) => {
      let permissions = fs::metadata(&old_exe_path)?.permissions();
      fs::set_permissions(&exe_path, permissions)?;

      if cfg!(windows) {
        // On windows you cannot replace the currently running executable.
        // so first we rename it to deno.old.exe
        fs::rename(&old_exe_path, &old_exe_path.with_extension("old.exe"))?;
      } else {
        fs::remove_file(&old_exe_path)?;
      }

      old_exe_path
    }
    Err(_) => {
      let deno_install = match env::var_os("DENO_INSTALL") {
        Some(deno_install) => PathBuf::from(deno_install),
        None => {
          println!("DENO_INSTALL is not defined, use $HOME/.deno/bin");
          let home_env_var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
          env::var_os(home_env_var).map(PathBuf::from).unwrap().join(".deno")
        }
      };

      let deno_bin_path = deno_install.join("bin");
      if !deno_bin_path.exists() {
        fs::create_dir_all(&deno_bin_path).unwrap();
      }

      let exe_ext = if cfg!(windows) { "exe" } else { "" };
      deno_bin_path.join("deno").with_extension(exe_ext)
    }
  };

  fs::copy(&exe_path, &old_exe_path)?;
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
