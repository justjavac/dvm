use crate::utils::check_is_deactivated;
use crate::{deno_bin_path, dvm_root};
use anyhow::{Ok, Result};

pub fn exec() -> Result<()> {
  let home = dvm_root();
  if check_is_deactivated() {
    println!("Dvm has already been deactivated, exiting.");
    return Ok(());
  }

  std::fs::write(home.join(".deactivated"), "").unwrap();
  std::fs::remove_file(deno_bin_path()).unwrap();

  println!("Dvm is now deacvated.");
  println!("Deno that was previously installed on your system will be activated now.");
  Ok(())
}
