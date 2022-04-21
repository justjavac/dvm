use crate::utils::deno_bin_path;
use crate::utils::is_china_mainland;
use crate::version::dvmrc_version;
use anyhow::Result;
use semver_parser::version::{parse as semver_parse, Version};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;

pub fn exec(version: Option<String>) -> Result<()> {
  fn get_latest_version() -> Result<Version> {
    println!("Checking for latest version");
    let response = if is_china_mainland() {
      tinyget::get("https://dl.deno.js.cn/release-latest.txt").send()?
    } else {
      tinyget::get("https://dl.deno.land/release-latest.txt").send()?
    };

    let body = response.as_str()?;
    let v = body.trim().replace('v', "");
    println!("The latest version is v{}", &v);
    Ok(semver_parse(&v).unwrap())
  }

  let version = version.unwrap_or_else(|| {
    println!("No version input detect, try to use version in .dvmrc file");
    dvmrc_version().unwrap_or_else(|| {
      println!("No version in .dvmrc file, try to use latest version");
      get_latest_version().unwrap().to_string()
    })
  });

  let used_version = match semver_parse(&version) {
    Ok(ver) => ver,
    Err(_) => {
      eprintln!("Invalid semver");
      std::process::exit(1)
    }
  };

  let new_exe_path = deno_bin_path(&used_version);

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
