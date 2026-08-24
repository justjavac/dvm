use crate::dvm_root;
use crate::utils::{check_is_deactivated, remove_deno_bin_link};
use anyhow::{Ok, Result};

pub fn exec() -> Result<()> {
  let home = dvm_root();
  if check_is_deactivated() {
    println!("Dvm has already been deactivated, exiting.");
    return Ok(());
  }

  std::fs::write(home.join(".deactivated"), "")?;
  remove_deno_bin_link()?;

  println!("Dvm is now deactivated.");
  println!("Deno that was previously installed on your system will be activated now.");
  Ok(())
}
