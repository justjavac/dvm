extern crate core;

mod cli;
mod commands;
mod configrc;
mod consts;
mod meta;
mod utils;
pub mod version;

use cfg_if::cfg_if;
use clap::CommandFactory;

use cli::{Cli, Commands};
use colored::Colorize;
use meta::DvmMeta;
use utils::{dvm_root, run_with_spinner};

use crate::meta::DEFAULT_ALIAS;
use crate::utils::deno_bin_path;

cfg_if! {
  if #[cfg(windows)] {
    use ctor::*;
    #[ctor]
    fn init() {
      output_vt100::try_init().ok();
    }
  }
}

pub fn main() {
  let mut meta = DvmMeta::new();

  let Ok(cli) = cli::cli_parse(&mut meta) else {
    return;
  };

  let result = match cli.command {
    Commands::Completions { shell } => commands::completions::exec(&mut Cli::command(), shell),
    Commands::Info => commands::info::exec(),
    Commands::Install { no_use, version } => run_with_spinner(
      format!("Installing {}", version.clone().unwrap_or_else(|| "latest".to_string())),
      "Installed".to_string(),
      |stop_with_error| match commands::install::exec(&meta, no_use, version) {
        Ok(ok) => Ok(ok),
        Err(err) => stop_with_error(format!("Failed to install: {}", err)),
      },
    ),
    Commands::List => commands::list::exec(),
    Commands::ListRemote => commands::list::exec_remote(),
    Commands::Uninstall { version } => commands::uninstall::exec(version),
    Commands::Use { version, write_local } => commands::use_version::exec(&mut meta, version, write_local),
    Commands::Alias { command } => commands::alias::exec(&mut meta, command),
    Commands::Activate => commands::activate::exec(&mut meta),
    Commands::Deactivate => commands::deactivate::exec(),
    Commands::Doctor => run_with_spinner(
      "Fixing...".to_string(),
      "All fixes applied, DVM is ready to use.".green().to_string(),
      |fail| match commands::doctor::exec(&mut meta) {
        Ok(ok) => Ok(ok),
        Err(err) => fail(format!("Failed to fix: {}", err)),
      },
    ),
    Commands::Upgrade { alias } => run_with_spinner(
      "Upgrading...".to_string(),
      "All alias have been upgraded.".to_string(),
      |fail| match commands::upgrade::exec(&mut meta, alias) {
        Ok(ok) => Ok(ok),
        Err(err) => fail(format!("Failed to upgrade: {}", err)),
      },
    ),

    Commands::Exec { command: _, version: _ } => {
      /* unused */
      Ok(())
    }
    Commands::Clean => {
      run_with_spinner(
        "Cleaning...".to_string(),
        "clean finished".to_string(),
        |fail| match commands::clean::exec(&mut meta) {
          Ok(ok) => Ok(ok),
          Err(err) => fail(format!("Failed to clean: {}", err)),
        },
      )
    }

    Commands::Registry { command } => commands::registry::exec(&mut meta, command),
    Commands::Update => run_with_spinner("Updating cache...".to_string(), "Update success".to_string(), |fail| {
      match commands::update::exec(&mut meta) {
        Ok(ok) => Ok(ok),
        Err(err) => fail(format!("Failed to update: {}", err)),
      }
    }),
  };

  if let Err(err) = result {
    eprintln!("\x1b[31merror:\x1b[39m: {}", err);
    std::process::exit(1);
  }
}
