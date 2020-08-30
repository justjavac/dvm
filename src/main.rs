extern crate clap;
extern crate regex;
extern crate reqwest;
extern crate url;

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
    DvmSubcommand::Install { no_use, version } => {
      commands::install::exec(no_use, version)
    }
    _ => unreachable!(),
  };

  if let Err(err) = result {
    eprintln!("{}", err);
    std::process::exit(1);
  }
}
