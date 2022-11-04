use std::env;

use clap::Parser;
use clap_complete::Shell;
use clap_derive::{Parser, Subcommand};

use crate::commands;
use crate::consts::{AFTER_HELP, COMPLETIONS_HELP};
use crate::meta::DvmMeta;

pub fn cli_parse(meta: &mut DvmMeta) -> Result<Cli, ()> {
  let args: Vec<String> = env::args().collect();
  if args.len() > 1 && args[1] == "exec" {
    if args.len() > 2 {
      let version: Option<String>;
      let exec_args: Vec<String>;
      if args[2] == "--version" || args[2] == "-V" {
        if args.len() > 3 {
          version = Some(args[3].clone());
          exec_args = args[4..].to_vec();
        } else {
          eprintln!("A version should be followed after {}", args[2]);
          std::process::exit(1)
        }
      } else if args[2].starts_with("--version=") || args[2].starts_with("-V=") {
        version = Some(
          args[2]
            .trim_start_matches("-V=")
            .trim_start_matches("--version=")
            .to_string(),
        );
        exec_args = args[3..].to_vec();
      } else {
        version = None;
        exec_args = args[2..].to_vec();
      }
      commands::exec::exec(meta, version, exec_args).unwrap();
    } else {
      // TODO(CGQAQ): print help
    }
    return Err(());
  }

  Ok(Cli::parse())
}

#[derive(Parser)]
#[clap(version, about)]
#[clap(after_help = AFTER_HELP)]
#[clap(propagate_version = true)]
pub struct Cli {
  #[clap(subcommand)]
  pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
  #[clap(about = "Generate shell completions")]
  #[clap(long_about=COMPLETIONS_HELP)]
  Completions {
    #[arg(value_enum)]
    shell: Shell,
  },

  #[clap(about = "Show dvm info.")]
  Info,

  #[clap(about = "Install deno executable to the given version.")]
  #[clap(visible_aliases=&["i", "add"])]
  Install {
    #[clap(long, help = "Only install to local, but not use")]
    no_use: bool,
    #[clap(help = "The version to install")]
    version: Option<String>,
  },

  #[clap(about = "List all installed versions")]
  #[clap(visible_aliases=&["ls", "ll", "la"])]
  List,

  #[clap(about = "List all released versions")]
  #[clap(visible_aliases=&["lr", "ls-remote"])]
  ListRemote,

  #[clap(about = "Uninstall a given version")]
  #[clap(visible_aliases=&["un", "unlink", "rm", "remove"])]
  Uninstall {
    #[clap(help = "The version to install")]
    version: Option<String>,
  },

  #[clap(about = "Use a given version or a semver range or a alias to the range.")]
  Use {
    #[clap(help = "The version, semver range or alias to use")]
    version: Option<String>,

    #[clap(
      short,
      long,
      help = "Writing the version to the .dvmrc file of the current directory if present"
    )]
    local: bool,
  },

  #[clap(about = "Set or unset an alias")]
  Alias {
    #[clap(subcommand)]
    command: AliasCommands,
  },

  #[clap(about = "Activate Dvm")]
  Activate,
  #[clap(about = "Deactivate Dvm")]
  Deactivate,

  #[clap(about = "Fixing dvm specific environment variables and other issues")]
  Doctor,

  #[clap(about = "Upgrade aliases to the latest version")]
  Upgrade {
    #[clap(help = "The alias to upgrade, upgrade all aliases if not present")]
    alias: Option<String>,
  },

  #[clap(about = "Execute deno command with a specific deno version")]
  Exec {
    #[clap(help = "The command given to deno")]
    command: Option<String>,

    #[clap(help = "The version to use", long, short)]
    deno_version: Option<String>,
  },

  #[clap(about = "Clean dvm cache")]
  Clean,

  #[clap(about = "Change registry that dvm fetch from")]
  Registry {
    #[clap(help = "The registry to be set, `official`, `cn`, or url you desired")]
    registry: Option<String>,
  },
}

#[derive(Subcommand)]
pub enum AliasCommands {
  #[clap(about = "Set an alias")]
  Set {
    #[clap(help = "Alias name to set")]
    name: String,
    #[clap(help = "Alias content")]
    content: String,
  },

  #[clap(about = "Unset an alias")]
  Unset {
    #[clap(help = "Alias name to unset")]
    name: String,
  },

  #[clap(about = "List all aliases")]
  List,
}
