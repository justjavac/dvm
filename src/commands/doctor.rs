use crate::configrc::{rc_clean, rc_init};
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
  set_env::check_or_set("DVM_DIR", home_path.to_str().unwrap()).unwrap();
  let path = set_env::get("PATH").unwrap();
  let looking_for = deno_bin_path().parent().unwrap().to_str().unwrap().to_string();
  let current = which::which("deno");

  if let Ok(current) = current {
    if current.to_str().unwrap().starts_with(&looking_for) {
      println!("{}", "DVM deno bin is already set correctly.".green());
    } else {
      set_env::prepend("PATH", looking_for.as_str()).unwrap();
      println!("{}", "Please restart your shell of choice to take effects.".red());
    }
  } else if !path.contains(looking_for.as_str()) {
    set_env::prepend("PATH", looking_for.as_str()).unwrap();
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
  rc_init().expect("init user-wide rc file failed");

  println!("{}", "All fixes applied, DVM is ready to use.".green());
  Ok(())
}
