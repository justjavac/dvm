use crate::configrc::{rc_clean, rc_fix};
use crate::consts::DVM_CACHE_PATH_PREFIX;
use anyhow::Result;
use colored::Colorize;
use std::fs;

use crate::meta::DvmMeta;
use crate::utils::{deno_bin_path, dvm_root, is_exact_version};

pub fn exec(meta: &mut DvmMeta) -> Result<()> {
  // Init enviroments if need
  // actually set DVM_DIR env var if not exist.
  let home_path = dvm_root();
  check_or_set_env("DVM_DIR", home_path.to_str().unwrap())?;
  let path = get_env("PATH")?;
  let looking_for = deno_bin_path().parent().unwrap().to_str().unwrap().to_string();
  let current = which::which("deno");

  if let Ok(current) = current {
    if current.to_str().unwrap().starts_with(&looking_for) {
      println!("{}", "DVM deno bin is already set correctly.".green());
    } else {
      prepend_env_path(looking_for.as_str())?;
      println!("{}", "Please restart your shell of choice to take effects.".red());
    }
  } else if !env_path_contains(&path, looking_for.as_str()) {
    prepend_env_path(looking_for.as_str())?;
    println!("{}", "Please restart your shell of choice to take effects.".red());
  }

  // migrating from old dvm cache.
  let cache_folder = home_path.join(DVM_CACHE_PATH_PREFIX);
  if !cache_folder.exists() {
    fs::create_dir_all(cache_folder)?;
  } else if !home_path.join(DVM_CACHE_PATH_PREFIX).is_dir() {
    fs::remove_file(cache_folder.clone())?;
    fs::create_dir_all(cache_folder)?;
  }
  let list = fs::read_dir(home_path).unwrap();
  for entry in list {
    let entry = entry.unwrap();
    let path = entry.path();
    if path.is_dir() {
      let name = path.file_name().unwrap().to_str().unwrap();
      if is_exact_version(name) {
        // move to `versions` subdir
        println!(
          "Found old dvm cache of version `{}`, migrating to new dvm cache location...",
          name
        );
        fs::rename(path.clone(), path.parent().unwrap().join("versions").join(name)).unwrap();
      }
    }
  }

  if dvm_root().exists() {
    super::use_version::exec(meta, None, false).unwrap();
  }

  // clean user-wide rc file
  rc_clean(true).expect("clean local rc file failed");
  rc_clean(false).expect("clean user-wide rc file failed");
  rc_fix().expect("fix rc file failed");

  println!("{}", "All fixes applied, DVM is ready to use.".green());
  Ok(())
}

#[cfg(not(windows))]
fn check_or_set_env(name: &str, value: &str) -> Result<()> {
  set_env::check_or_set(name, value).map_err(Into::into)
}

#[cfg(not(windows))]
fn get_env(name: &str) -> Result<String> {
  set_env::get(name).map_err(Into::into)
}

#[cfg(not(windows))]
fn prepend_env_path(value: &str) -> Result<()> {
  set_env::prepend("PATH", value).map_err(Into::into)
}

#[cfg(not(windows))]
fn env_path_contains(path: &str, value: &str) -> bool {
  path.contains(value)
}

#[cfg(windows)]
fn check_or_set_env(name: &str, value: &str) -> Result<()> {
  if std::env::var_os(name).is_none() {
    set_user_env(name, value)?;
  }
  Ok(())
}

#[cfg(windows)]
fn get_env(name: &str) -> Result<String> {
  std::env::var(name).map_err(Into::into)
}

#[cfg(windows)]
fn prepend_env_path(value: &str) -> Result<()> {
  let user_path = get_user_env("Path")?.unwrap_or_default();
  let new_user_path = prepend_path_value(&user_path, value);

  set_user_env("Path", &new_user_path)?;

  let process_path = std::env::var("PATH").unwrap_or_default();
  std::env::set_var("PATH", prepend_path_value(&process_path, value));

  Ok(())
}

#[cfg(windows)]
fn get_user_env(name: &str) -> Result<Option<String>> {
  let output = std::process::Command::new("powershell.exe")
    .arg("-NoLogo")
    .arg("-NoProfile")
    .arg("-NonInteractive")
    .arg("-Command")
    .arg("[Environment]::GetEnvironmentVariable($args[0], 'User')")
    .arg(name)
    .output()?;

  if !output.status.success() {
    anyhow::bail!("Failed to read user environment variable {}", name);
  }

  let value = String::from_utf8(output.stdout)?.trim().to_string();
  Ok((!value.is_empty()).then_some(value))
}

#[cfg(windows)]
fn set_user_env(name: &str, value: &str) -> Result<()> {
  let status = std::process::Command::new("powershell.exe")
    .arg("-NoLogo")
    .arg("-NoProfile")
    .arg("-NonInteractive")
    .arg("-Command")
    .arg("[Environment]::SetEnvironmentVariable($args[0], $args[1], 'User')")
    .arg(name)
    .arg(value)
    .status()?;

  if !status.success() {
    anyhow::bail!("Failed to set user environment variable {}", name);
  }

  std::env::set_var(name, value);
  Ok(())
}

#[cfg(windows)]
fn path_contains(path: &str, value: &str) -> bool {
  path.split(';').any(|item| item.eq_ignore_ascii_case(value))
}

#[cfg(windows)]
fn env_path_contains(path: &str, value: &str) -> bool {
  path_contains(path, value)
}

#[cfg(windows)]
fn prepend_path_value(path: &str, value: &str) -> String {
  let rest = path
    .split(';')
    .filter(|item| !item.is_empty() && !item.eq_ignore_ascii_case(value))
    .collect::<Vec<_>>()
    .join(";");

  if rest.is_empty() {
    value.to_string()
  } else {
    format!("{};{}", value, rest)
  }
}

#[cfg(all(test, windows))]
mod tests {
  use super::*;

  #[test]
  fn prepend_path_value_moves_existing_entry_to_front() {
    assert_eq!(
      prepend_path_value(
        "C:\\Windows;C:\\Users\\me\\.dvm\\bin;C:\\Tools",
        "C:\\Users\\me\\.dvm\\bin"
      ),
      "C:\\Users\\me\\.dvm\\bin;C:\\Windows;C:\\Tools"
    );
    assert_eq!(
      prepend_path_value("C:\\Windows;C:\\Tools", "C:\\Users\\me\\.dvm\\bin"),
      "C:\\Users\\me\\.dvm\\bin;C:\\Windows;C:\\Tools"
    );
  }
}
