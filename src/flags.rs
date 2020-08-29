// Copyright 2018-2020 the Deno authors. All rights reserved. MIT license.
// Copyright 2020 the Dvm authors. All rights reserved. MIT license.

use clap::{App, AppSettings, Arg, SubCommand};

#[derive(Clone, Debug, PartialEq)]
pub enum DvmSubcommand {
  Completions {
    buf: Box<[u8]>,
  },
  Help,
  Upgrade {
    dry_run: bool,
    force: bool,
    version: Option<String>,
  },
}

impl Default for DvmSubcommand {
  fn default() -> DvmSubcommand {
    DvmSubcommand::Help
  }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Flags {
  /// Vector of CLI arguments - these are user script arguments, all Deno
  /// specific flags are removed.
  pub argv: Vec<String>,
  pub subcommand: DvmSubcommand,
}

static DENO_HELP: &str =
  "Deno Version Manager - Easy way to manage multiple active deno versions.";

/// Main entry point for parsing deno's command line flags.
/// Exits the process on error.
pub fn flags_from_vec(args: Vec<String>) -> Flags {
  match flags_from_vec_safe(args) {
    Ok(flags) => flags,
    Err(err) => err.exit(),
  }
}

/// Same as flags_from_vec but does not exit on error.
pub fn flags_from_vec_safe(args: Vec<String>) -> clap::Result<Flags> {
  let app = clap_root();
  let matches = app.get_matches_from_safe(args)?;

  let mut flags = Flags::default();

  if let Some(m) = matches.subcommand_matches("completions") {
    completions_parse(&mut flags, m);
  } else if let Some(m) = matches.subcommand_matches("upgrade") {
    upgrade_parse(&mut flags, m);
  }

  Ok(flags)
}

fn clap_root<'a, 'b>() -> App<'a, 'b> {
  clap::App::new("dvm")
    .bin_name("dvm")
    .global_settings(&[
      AppSettings::UnifiedHelpMessage,
      AppSettings::ColorNever,
      AppSettings::VersionlessSubcommands,
    ])
    .set_term_width(0)
    .version(crate::version::DENO)
    .subcommand(completions_subcommand())
    .subcommand(upgrade_subcommand())
    .long_about(DENO_HELP)
}

fn completions_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  let shell: &str = matches.value_of("shell").unwrap();
  let mut buf: Vec<u8> = vec![];
  use std::str::FromStr;
  clap_root().gen_completions_to(
    "dvm",
    clap::Shell::from_str(shell).unwrap(),
    &mut buf,
  );

  flags.subcommand = DvmSubcommand::Completions {
    buf: buf.into_boxed_slice(),
  };
}

fn upgrade_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  let dry_run = matches.is_present("dry-run");
  let force = matches.is_present("force");
  let version = matches.value_of("version").map(|s| s.to_string());
  flags.subcommand = DvmSubcommand::Upgrade {
    dry_run,
    force,
    version,
  };
}

fn completions_subcommand<'a, 'b>() -> App<'a, 'b> {
  SubCommand::with_name("completions")
    .setting(AppSettings::DisableHelpSubcommand)
    .arg(
      Arg::with_name("shell")
        .possible_values(&clap::Shell::variants())
        .required(true),
    )
    .about("Generate shell completions")
    .long_about(
      "Output shell completion script to standard output.
  dvm completions bash > /usr/local/etc/bash_completion.d/dvm.bash
  source /usr/local/etc/bash_completion.d/dvm.bash",
    )
}

fn upgrade_subcommand<'a, 'b>() -> App<'a, 'b> {
  SubCommand::with_name("upgrade")
    .about("Upgrade deno executable to given version")
    .long_about(
      "Upgrade deno executable to the given version.
Defaults to latest.

The version is downloaded from
https://github.com/denoland/deno/releases
and is used to replace the current executable.

If you want to not replace the current Deno executable but instead download an
update to a different location, use the --output flag
  deno upgrade --output $HOME/my_deno",
    )
    .arg(
      Arg::with_name("version")
        .help("The version to upgrade to")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("dry-run")
        .long("dry-run")
        .help("Perform all checks without replacing old exe"),
    )
    .arg(
      Arg::with_name("force")
        .long("force")
        .short("f")
        .help("Replace current exe even if not out-of-date"),
    )
}

#[cfg(test)]
/// Creates vector of strings, Vec<String>
macro_rules! svec {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn upgrade() {
    let r =
      flags_from_vec_safe(svec!["deno", "upgrade", "--dry-run", "--force"]);
    let flags = r.unwrap();
    assert_eq!(
      flags,
      Flags {
        subcommand: DvmSubcommand::Upgrade {
          force: true,
          dry_run: true,
          version: None,
        },
        ..Flags::default()
      }
    );
  }

  #[test]
  fn version() {
    let r = flags_from_vec_safe(svec!["deno", "--version"]);
    assert_eq!(r.unwrap_err().kind, clap::ErrorKind::VersionDisplayed);
    let r = flags_from_vec_safe(svec!["deno", "-V"]);
    assert_eq!(r.unwrap_err().kind, clap::ErrorKind::VersionDisplayed);
  }
}
