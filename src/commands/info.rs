use anyhow::Result;
use std::string::String;

use crate::utils;
use crate::version;

pub fn exec() -> Result<()> {
  println!(
    "dvm {}\ndeno {}\ndvm root {}",
    version::DVM,
    version::get_current_version().unwrap_or_else(|| String::from("-")),
    utils::get_dvm_root().as_path().to_string_lossy(),
  );
  Ok(())
}
