use crate::configrc::{rc_clean, rc_fix};
use crate::consts::DVM_CACHE_PATH_PREFIX;
use anyhow::Result;
use colored::Colorize;
use std::fs;

use crate::meta::DvmMeta;
use crate::utils::{deno_resolution, dvm_bin_dir, dvm_bin_on_path, dvm_root, is_exact_version, DenoResolution};

pub fn exec(meta: &mut DvmMeta) -> Result<()> {
  // Clean up stale version mappings before doing anything else — doctor is
  // where integrity checks belong, not on every command startup.
  let _ = meta.cleanup_stale_mappings()?;

  // Init enviroments if need
  // actually set DVM_DIR env var if not exist.
  let home_path = dvm_root();
  let home_str = home_path
    .to_str()
    .ok_or_else(|| anyhow::anyhow!("DVM_DIR path contains non-UTF-8 bytes"))?;
  check_or_set_env("DVM_DIR", home_str)?;
  let bin_dir = dvm_bin_dir();
  let looking_for = bin_dir
    .to_str()
    .ok_or_else(|| anyhow::anyhow!("dvm bin path contains non-UTF-8 bytes"))?
    .to_string();

  // Share the resolution with `dvm use` so both agree on what "dvm's deno wins
  // on PATH" means: a plain string prefix would also accept a sibling directory
  // such as `.dvm/bin2`, and it ignores Windows' case-insensitive paths.
  match deno_resolution() {
    DenoResolution::Dvm => println!("{}", "DVM deno bin is already set correctly.".green()),
    DenoResolution::Shadowed(shadowing) => {
      println!("`deno` currently resolves to {}", shadowing.display());
      prepend_env_path(looking_for.as_str())?;
      println!("{}", "Please restart your shell of choice to take effects.".red());
    }
    DenoResolution::NotOnPath => {
      if !dvm_bin_on_path() {
        prepend_env_path(looking_for.as_str())?;
        println!("{}", "Please restart your shell of choice to take effects.".red());
      }
    }
  }

  // migrating from old dvm cache.
  let cache_folder = home_path.join(DVM_CACHE_PATH_PREFIX);
  if !cache_folder.exists() {
    fs::create_dir_all(cache_folder)?;
  } else if !home_path.join(DVM_CACHE_PATH_PREFIX).is_dir() {
    fs::remove_file(cache_folder.clone())?;
    fs::create_dir_all(cache_folder)?;
  }
  for entry in fs::read_dir(&home_path)? {
    let path = entry?.path();
    if path.is_dir() {
      let Some(name) = path.file_name().and_then(|it| it.to_str()) else {
        continue;
      };
      if is_exact_version(name) {
        // move to `versions` subdir
        println!(
          "Found old dvm cache of version `{}`, migrating to new dvm cache location...",
          name
        );
        move_dir(&path, &home_path.join(DVM_CACHE_PATH_PREFIX).join(name))?;
      }
    }
  }

  if dvm_root().exists() {
    super::use_version::exec(meta, None, false)?;
  }

  // clean user-wide rc file
  rc_clean(true)?;
  rc_clean(false)?;
  rc_fix()?;

  // The success line is the spinner's finish message in `main`, printing it
  // here too showed it twice.
  Ok(())
}

#[cfg(not(windows))]
fn check_or_set_env(name: &str, value: &str) -> Result<()> {
  // Already set in the current process — nothing to do.
  if std::env::var_os(name).is_some() {
    return Ok(());
  }

  // Persist the variable in the user's shell config so future shells pick it up.
  let export_line = format!("export {}=\"{}\"", name, value);
  add_to_shell_config(&export_line)?;

  // Also set it for the current process so subsequent checks in this run pass.
  std::env::set_var(name, value);
  Ok(())
}

#[cfg(not(windows))]
fn prepend_env_path(value: &str) -> Result<()> {
  // Already on PATH in the current process — nothing to do.
  if let Ok(path) = std::env::var("PATH") {
    if path.split(':').any(|p| p == value) {
      return Ok(());
    }
  }

  // Persist the PATH entry in the user's shell config.
  let export_line = format!("export PATH=\"{}:$PATH\"", value);
  add_to_shell_config(&export_line)?;

  // Also prepend it for the current process.
  let current_path = std::env::var("PATH").unwrap_or_default();
  let new_path = if current_path.is_empty() {
    value.to_string()
  } else {
    format!("{}:{}", value, current_path)
  };
  std::env::set_var("PATH", new_path);
  Ok(())
}

/// Append `line` to the user's shell configuration file(s) if it is not
/// already present.  The shell is detected from the `$SHELL` environment
/// variable, falling back to common config files when unknown.
#[cfg(not(windows))]
fn add_to_shell_config(line: &str) -> Result<()> {
  use std::io::Write;

  let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
  let shell = std::env::var("SHELL").unwrap_or_default();

  // Build a list of candidate config files in priority order.
  let config_files: Vec<std::path::PathBuf> = if shell.contains("zsh") {
    vec![home.join(".zshrc")]
  } else if shell.contains("fish") {
    vec![home.join(".config/fish/config.fish")]
  } else if shell.contains("bash") {
    #[allow(unused_mut)]
    let mut files = vec![home.join(".bashrc")];
    // On macOS, login shells read .bash_profile instead of .bashrc.
    #[cfg(target_os = "macos")]
    files.push(home.join(".bash_profile"));
    files
  } else {
    // Unknown shell — try the most common ones.
    #[allow(unused_mut)]
    let mut files = vec![home.join(".bashrc"), home.join(".zshrc")];
    #[cfg(target_os = "macos")]
    files.push(home.join(".bash_profile"));
    files
  };

  let mut added = false;

  for config_file in &config_files {
    // Only touch files that already exist, so we don't create random config
    // files for shells the user doesn't use.
    if config_file.exists() {
      let content = std::fs::read_to_string(config_file)?;
      if !content.lines().any(|l| l.trim() == line.trim()) {
        let mut file = std::fs::OpenOptions::new().append(true).open(config_file)?;
        writeln!(file, "{}", line)?;
        added = true;
      } else {
        // Already present in this file — consider it done.
        added = true;
      }
    }
  }

  // No existing config file was found — create the primary one for the
  // detected shell (or .bashrc as a fallback).
  if !added {
    if let Some(primary) = config_files.first() {
      if let Some(parent) = primary.parent() {
        std::fs::create_dir_all(parent)?;
      }
      let mut file = std::fs::OpenOptions::new().create(true).append(true).open(primary)?;
      writeln!(file, "{}", line)?;
    }
  }

  Ok(())
}

#[cfg(windows)]
fn check_or_set_env(name: &str, value: &str) -> Result<()> {
  if std::env::var_os(name).is_none() {
    set_user_env(name, value)?;
  }
  Ok(())
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

/// Move a directory from `src` to `dst`.
///
/// Tries `fs::rename` first (fast, atomic within a filesystem).  Falls back
/// to a recursive copy + delete when the source and destination are on
/// different filesystems (EXDEV error).
fn move_dir(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
  match fs::rename(src, dst) {
    Ok(()) => Ok(()),
    Err(e) if is_cross_device_error(&e) => {
      // Cross-filesystem move — fall back to copy + remove
      copy_dir_recursive(src, dst)?;
      fs::remove_dir_all(src)?;
      Ok(())
    }
    Err(e) => Err(e.into()),
  }
}

/// Returns `true` if the I/O error indicates a cross-device link (EXDEV),
/// meaning `fs::rename` cannot move the file because source and destination
/// are on different filesystems.
///
/// Uses `raw_os_error` for compatibility with Rust versions before 1.85
/// (which stabilized `ErrorKind::CrossesDevices`).  On Unix, EXDEV is
/// defined as 18 by POSIX.
#[cfg(unix)]
fn is_cross_device_error(e: &std::io::Error) -> bool {
  e.raw_os_error() == Some(18) // EXDEV
}

#[cfg(not(unix))]
fn is_cross_device_error(_e: &std::io::Error) -> bool {
  false
}

/// Recursively copy a directory from `src` to `dst`.
///
/// Creates `dst` and copies all entries (files and subdirectories) into it.
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> Result<()> {
  fs::create_dir_all(dst)?;
  for entry in fs::read_dir(src)? {
    let entry = entry?;
    let path = entry.path();
    let file_name = entry.file_name();
    let dst_path = dst.join(file_name);

    if path.is_dir() {
      copy_dir_recursive(&path, &dst_path)?;
    } else {
      fs::copy(&path, &dst_path)?;
    }
  }
  Ok(())
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
