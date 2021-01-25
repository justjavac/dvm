// Copyright 2018-2020 the Deno authors. All rights reserved. MIT license.
// Copyright 2020 justjavac. All rights reserved. MIT license.
use clap::{App, AppSettings, Arg, SubCommand};

#[derive(Clone, Debug, PartialEq)]
pub enum DvmSubcommand {
  Completions {
    buf: Box<[u8]>,
  },
  Help,
  Info,
  Install {
    no_use: bool,
    version: Option<String>,
  },
  List,
  Use {
    version: Option<String>,
  },
  Uninstall {
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

static DVM_HELP: &str =
  "Deno Version Manager - Easy way to manage multiple active deno versions.";

static DVM_EXAMPLE: &str = "Example:
  dvm install 1.3.2     Install v1.3.2 release
  dvm install           Install the latest available version
  dvm use 1.0.0         Use v1.0.0 release";

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
  } else if let Some(m) = matches.subcommand_matches("info") {
    info_parse(&mut flags, m);
  } else if let Some(m) = matches.subcommand_matches("install") {
    install_parse(&mut flags, m);
  } else if let Some(m) = matches.subcommand_matches("list") {
    list_parse(&mut flags, m);
  } else if let Some(m) = matches.subcommand_matches("use") {
    use_parse(&mut flags, m);
  } else if let Some(m) = matches.subcommand_matches("uninstall") {
    uninstall_parse(&mut flags, m);
  } else {
    info_parse(&mut flags, &matches);
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
    .version(crate::version::DVM)
    .subcommand(completions_subcommand())
    .subcommand(info_subcommand())
    .subcommand(install_subcommand())
    .subcommand(list_subcommand())
    .subcommand(use_subcommand())
    .subcommand(uninstall_subcommand())
    .long_about(DVM_HELP)
    .after_help(DVM_EXAMPLE)
}

fn info_parse(flags: &mut Flags, _matches: &clap::ArgMatches) {
  flags.subcommand = DvmSubcommand::Info;
}

fn list_parse(flags: &mut Flags, _matches: &clap::ArgMatches) {
  flags.subcommand = DvmSubcommand::List;
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

fn install_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  let no_use = matches.is_present("no-use");
  let version = matches.value_of("version").map(|s| s.to_string());
  flags.subcommand = DvmSubcommand::Install { no_use, version };
}

fn use_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  let version = matches.value_of("version").map(|s| s.to_string());
  flags.subcommand = DvmSubcommand::Use { version };
}

fn uninstall_parse(flags: &mut Flags, matches: &clap::ArgMatches) {
  let version = matches.value_of("version").map(|s| s.to_string());
  flags.subcommand = DvmSubcommand::Uninstall { version };
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

fn list_subcommand<'a, 'b>() -> App<'a, 'b> {
  SubCommand::with_name("list")
    .visible_alias("ls")
    .about("List installed versions, matching a given <version> if provided")
    .long_about(
      "List installed versions, matching a given <version> if provided",
    )
}

fn info_subcommand<'a, 'b>() -> App<'a, 'b> {
  SubCommand::with_name("info")
    .about("Show dvm info")
    .long_about("Show dvm info.")
}

fn install_subcommand<'a, 'b>() -> App<'a, 'b> {
  SubCommand::with_name("install")
    .visible_alias("i")
    .about("Install deno executable to given version")
    .long_about(
      "Install deno executable to the given version.
Defaults to latest.",
    )
    .arg(
      Arg::with_name("version")
        .help("The version to install")
        .takes_value(true),
    )
    .arg(
      Arg::with_name("no-use")
        .long("no-use")
        .help("Only install to local, but not use"),
    )
}

fn use_subcommand<'a, 'b>() -> App<'a, 'b> {
  SubCommand::with_name("use")
    .about("Use a given version")
    .long_about("Use a given version.")
    .arg(
      Arg::with_name("version")
        .help("The version to use")
        .takes_value(true),
    )
}

fn uninstall_subcommand<'a, 'b>() -> App<'a, 'b> {
  SubCommand::with_name("uninstall")
    .visible_alias("rm")
    .about("Uninstall a given version")
    .long_about("Uninstall a given version.")
    .arg(
      Arg::with_name("version")
        .help("The version to uninstall")
        .takes_value(true),
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
  fn install() {
    let r = flags_from_vec_safe(svec!["deno", "install", "--no-use"]);
    let flags = r.unwrap();
    assert_eq!(
      flags,
      Flags {
        subcommand: DvmSubcommand::Install {
          no_use: true,
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
