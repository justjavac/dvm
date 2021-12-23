use anyhow::Result;
use clap::App;
use clap_generate::{generate, Shell};

pub fn exec(app: &mut App, shell: Shell) -> Result<()> {
  generate(shell, app, "dvm", &mut std::io::stdout());
  Ok(())
}
