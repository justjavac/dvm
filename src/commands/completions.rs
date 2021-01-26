use anyhow::{anyhow, Result};

use std::io::Write;

pub fn exec(buf: &[u8]) -> Result<()> {
  match std::io::stdout().write_all(buf) {
    Ok(()) => Ok(()),
    Err(e) => Err(anyhow!("{}", e)),
  }
}
