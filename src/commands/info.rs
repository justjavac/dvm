use crate::utils;
use crate::version;
use anyhow::Result;
use std::string::String;

pub fn exec() -> Result<()> {
  println!(
    "dvm {}\ndeno {}\ndvm root {}",
    version::DVM,
    version::current_version().unwrap_or_else(|| String::from("-")),
    utils::dvm_root().as_path().to_string_lossy(),
  );
  Ok(())
}
