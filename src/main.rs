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
      || commands::install::exec(&meta, no_use, version)
        .map_err(|err| anyhow::anyhow!("Failed to install: {}", err)),
    ),
    Commands::List => commands::list::exec(),
    Commands::ListRemote => commands::list::exec_remote(),
    Commands::Uninstall { version } => commands::uninstall::exec(&mut meta, version),
    Commands::Use { version, write_local } => commands::use_version::exec(&mut meta, version, write_local),
    Commands::Alias { command } => commands::alias::exec(&mut meta, command),
    Commands::Activate => commands::activate::exec(&mut meta),
    Commands::Deactivate => commands::deactivate::exec(),
    Commands::Doctor => run_with_spinner(
      "Fixing...".to_string(),
      "All fixes applied, DVM is ready to use.".green().to_string(),
      || commands::doctor::exec(&mut meta)
        .map_err(|err| anyhow::anyhow!("Failed to fix: {}", err)),
    ),
    Commands::Upgrade { alias } => run_with_spinner(
      "Upgrading...".to_string(),
      "All alias have been upgraded.".to_string(),
      || commands::upgrade::exec(&mut meta, alias)
        .map_err(|err| anyhow::anyhow!("Failed to upgrade: {}", err)),
    ),

    // `dvm exec` is handled in `cli_parse` *before* clap parsing,
    // because every argument after the version must be forwarded to deno
    // verbatim — something clap cannot express.  The `Exec` variant still
    // exists in the clap definition so it appears in `--help` output, but
    // this arm is never reached.
    Commands::Exec { .. } => unreachable!("exec handled in cli_parse before clap"),

    Commands::Clean => {
      run_with_spinner(
        "Cleaning...".to_string(),
        "clean finished".to_string(),
        || commands::clean::exec(&mut meta)
          .map_err(|err| anyhow::anyhow!("Failed to clean: {}", err)),
      )
    }

    Commands::Registry { command } => commands::registry::exec(command),
    Commands::Update => run_with_spinner("Updating cache...".to_string(), "Update success".to_string(), || {
      commands::update::exec(&mut meta)
        .map_err(|err| anyhow::anyhow!("Failed to update: {}", err))
    }),
  };

  if let Err(err) = result {
    utils::print_error(&err);
    std::process::exit(1);
  }
}
