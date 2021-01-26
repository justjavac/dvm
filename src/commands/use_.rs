use anyhow::Result;
use semver_parser::version::{parse as semver_parse, Version};
use which::which;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::utils::get_exe_path;

pub fn exec(version: Option<String>) -> Result<()> {
  let used_version = match version {
    Some(used_version) => match semver_parse(&used_version) {
      Ok(ver) => ver,
      Err(_) => {
        eprintln!("Invalid semver");
        std::process::exit(1)
      }
    },
    None => unimplemented!(),
  };

  let new_exe_path = get_exe_path(&used_version);

  if !new_exe_path.exists() {
    eprintln!(
      "deno v{} is not installed. Use `dvm install {}` to install it first.",
      used_version, used_version
    );
    std::process::exit(1)
  }

  use_this_bin_path(&new_exe_path, &used_version)?;
  Ok(())
}

pub fn use_this_bin_path(exe_path: &PathBuf, version: &Version) -> Result<()> {
  check_exe(&exe_path, &version)?;

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
          env::var_os(home_env_var).map(PathBuf::from).unwrap()
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
  println!("now use deno {}", version);
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
