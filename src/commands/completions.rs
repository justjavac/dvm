use anyhow::Result;
use clap::App;
use clap_complete::{generate, Shell};

pub fn exec(app: &mut App, shell: Shell) -> Result<()> {
  generate(shell, app, "dvm", &mut std::io::stdout());
  Ok(())
}
