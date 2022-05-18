use anyhow::Result;
use colored::Colorize;

use crate::{
  meta::DvmMeta,
  utils::{deno_bin_path, dvm_root},
};

pub fn exec(meta: &mut DvmMeta) -> Result<()> {
  // Init enviroments if need
  // actually set DVM_DIR env var if not exist.
  let home_path = dvm_root();
  set_env::check_or_set("DVM_DIR", home_path.to_str().unwrap()).unwrap();
  let path = set_env::get("PATH").unwrap();
  let looking_for = deno_bin_path().parent().unwrap().to_str().unwrap().to_string();
  if !path.contains(looking_for.as_str()) {
    set_env::prepend("PATH", looking_for.as_str()).unwrap();
    println!("{}", "Please restart your shell of choice to take effects.".red());
  }

  if !dirs::home_dir().unwrap().join(".dvmrc").exists() {
    super::use_version::exec(meta, None).unwrap();
  }

  println!("{}", "All fixes applied, DVM is ready to use.".green());
  Ok(())
}
