use anyhow::Result;
use clap::Command;
use clap_complete::{generate, Shell};

pub fn exec(app: &mut Command, shell: Shell) -> Result<()> {
  generate(shell, app, "dvm", &mut std::io::stdout());
  Ok(())
}
