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
use utils::{dvm_root, run_with_spinner, with_dvm_lock};

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

  let cli = cli::cli_parse();
  let command = cli.command;

  // `dvm exec` is a transparent wrapper around deno — it must forward
  // deno's exit code verbatim so scripts and CI see the right status.
  // Handled before the main match because exec calls process::exit directly
  // (its return type is effectively `!`, not `Result<()>`).
  if let Commands::Exec { version, args } = command {
    commands::exec::exec(&mut meta, version, args).unwrap_or_else(|err| {
      utils::print_error(&err);
      std::process::exit(1);
    });
    // exec never returns on success (it calls process::exit with deno's code)
    unreachable!("exec should have exited the process");
  }

  let result = match command {
    Commands::Completions { shell } => commands::completions::exec(&mut Cli::command(), shell),
    Commands::Info => commands::info::exec(),
    Commands::Install { no_use, version } => with_dvm_lock(|| {
      run_with_spinner(
        format!("Installing {}", version.clone().unwrap_or_else(|| "latest".to_string())),
        "Installed".to_string(),
        || commands::install::exec(&meta, no_use, version)
          .map_err(|err| anyhow::anyhow!("Failed to install: {}", err)),
      )
    }),
    Commands::List { remote } => {
      if remote {
        commands::list::exec_remote()
      } else {
        commands::list::exec()
      }
    }
    Commands::ListRemote => commands::list::exec_remote(),
    Commands::Uninstall { version } => with_dvm_lock(|| commands::uninstall::exec(&mut meta, version)),
    Commands::Use { version, write_local } => {
      with_dvm_lock(|| commands::use_version::exec(&mut meta, version, write_local))
    }
    Commands::Alias { command } => with_dvm_lock(|| commands::alias::exec(&mut meta, command)),
    Commands::Activate => with_dvm_lock(|| commands::activate::exec(&mut meta)),
    Commands::Deactivate => with_dvm_lock(commands::deactivate::exec),
    Commands::Doctor => with_dvm_lock(|| {
      run_with_spinner(
        "Fixing...".to_string(),
        "All fixes applied, DVM is ready to use.".green().to_string(),
        || commands::doctor::exec(&mut meta)
          .map_err(|err| anyhow::anyhow!("Failed to fix: {}", err)),
      )
    }),
    Commands::Upgrade { alias, dry_run } => with_dvm_lock(|| {
      run_with_spinner(
        "Upgrading...".to_string(),
        "All alias have been upgraded.".to_string(),
        || commands::upgrade::exec(&mut meta, alias, dry_run)
          .map_err(|err| anyhow::anyhow!("Failed to upgrade: {}", err)),
      )
    }),
    // exec handled above (before the match)
    Commands::Exec { .. } => unreachable!(),

    Commands::Clean { yes } => with_dvm_lock(|| {
      run_with_spinner(
        "Cleaning...".to_string(),
        "clean finished".to_string(),
        || commands::clean::exec(&mut meta, yes)
          .map_err(|err| anyhow::anyhow!("Failed to clean: {}", err)),
      )
    }),

    Commands::Registry { command } => commands::registry::exec(command),
    Commands::Update => with_dvm_lock(|| {
      run_with_spinner("Updating cache...".to_string(), "Update success".to_string(), || {
        commands::update::exec(&mut meta)
          .map_err(|err| anyhow::anyhow!("Failed to update: {}", err))
      })
    }),

    Commands::Which => commands::which::exec(),
  };

  if let Err(err) = result {
    utils::print_error(&err);
    std::process::exit(1);
  }
}
