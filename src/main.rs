extern crate clap;
extern crate regex;
extern crate reqwest;
extern crate url;

pub mod commands;
mod flags;
pub mod version;

use commands::upgrade::upgrade_command;
use flags::DvmSubcommand;

use std::env;

pub fn main() {
  let args: Vec<String> = env::args().collect();
  let flags = flags::flags_from_vec(args);

  let result = match flags.subcommand {
    DvmSubcommand::Upgrade {
      force,
      dry_run,
      version,
    } => upgrade_command(dry_run, force, version),
    _ => unreachable!(),
  };

  if let Err(err) = result {
    eprintln!("{}", err);
    std::process::exit(1);
  }
}
