extern crate anyhow;
extern crate clap;
// extern crate getopts;
extern crate semver_parser;
extern crate tempfile;
extern crate tinyget;
extern crate which;

mod commands;
mod flags;
mod utils;
pub mod version;

use flags::DvmSubcommand;

use std::env;

pub fn main() {
  let args: Vec<String> = env::args().collect();
  let flags = flags::flags_from_vec(args);

  let result = match flags.subcommand {
    DvmSubcommand::Completions { buf } => commands::completions::exec(&buf),
    DvmSubcommand::Info {} => commands::info::exec(),
    DvmSubcommand::Install { no_use, version } => {
      commands::install::exec(no_use, version)
    }
    DvmSubcommand::List {} => commands::list::exec(),
    DvmSubcommand::ListRemote {} => commands::list::exec_remote(),
    DvmSubcommand::Use { version } => commands::use_::exec(version),
    DvmSubcommand::Uninstall { version } => commands::uninstall::exec(version),
    _ => unreachable!(),
  };

  if let Err(err) = result {
    eprintln!("{}", err);
    std::process::exit(1);
  }
}
