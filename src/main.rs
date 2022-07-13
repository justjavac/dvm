extern crate core;

mod commands;
mod consts;
mod meta;
mod utils;
pub mod version;

use std::env;

use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use clap_derive::{Parser, Subcommand};
use meta::DvmMeta;
use utils::dvm_root;

use crate::meta::DEFAULT_ALIAS;
use crate::utils::deno_bin_path;
#[cfg(windows)]
use ctor::*;

#[cfg(windows)]
#[ctor]
fn init() {
  output_vt100::try_init().ok();
}

static AFTER_HELP: &str = "\x1b[33mEXAMPLE:\x1b[39m
  dvm install 1.3.2     Install v1.3.2 release
  dvm install           Install the latest available version
  dvm use 1.0.0         Use v1.0.0 release
  dvm use latest        Use the latest alias that comes with dvm, equivalent to *
  dvm use ^1.0.0        Use 1.x version (~1.0.0, >=1.0.0 are supported as well)
  
\x1b[33mNOTE:\x1b[39m
  To remove, delete, or uninstall dvm - just remove the \x1b[36m`$DVM_DIR`\x1b[39m folder (usually \x1b[36m`~/.dvm`\x1b[39m)";

static COMPLETIONS_HELP: &str = "Output shell completion script to standard output.
  \x1b[35m
  dvm completions bash > /usr/local/etc/bash_completion.d/dvm.bash
  source /usr/local/etc/bash_completion.d/dvm.bash\x1b[39m";

#[derive(Parser)]
#[clap(version, about)]
#[clap(after_help = AFTER_HELP)]
#[clap(propagate_version = true)]
struct Cli {
  #[clap(subcommand)]
  command: Commands,
}

#[derive(Subcommand)]
enum Commands {
  #[clap(about = "Generate shell completions")]
  #[clap(long_about=COMPLETIONS_HELP)]
  Completions {
    #[clap(arg_enum)]
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

  #[clap(about = "List installed versions, matching a given <version> if provided")]
  #[clap(visible_aliases=&["ls", "ll", "la"])]
  List,

  #[clap(about = "List released versions")]
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

pub fn main() {
  let mut meta = DvmMeta::new();

  let args: Vec<String> = env::args().collect();
  if args.len() > 1 {
    if args[1] == "exec" {
      if args.len() > 2 {
        let version: Option<String>;
        if args[2] == "--version" || args[2] == "-V" {
          if args.len() > 3 {
            version = Some(args[3].clone());
            commands::exec::exec(&mut meta, version, args[4..].to_vec()).unwrap();
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
          commands::exec::exec(&mut meta, version, args[3..].to_vec()).unwrap();
        } else {
          version = None;
          commands::exec::exec(&mut meta, version, args[2..].to_vec()).unwrap();
        }
      } else {
        // TODO(CGQAQ): print help
      }
      return;
    }
  }

  println!("hello?");

  let cli = Cli::parse();

  let result = match cli.command {
    Commands::Completions { shell } => commands::completions::exec(&mut Cli::command(), shell),
    Commands::Info => commands::info::exec(),
    Commands::Install { no_use, version } => commands::install::exec(&meta, no_use, version),
    Commands::List => commands::list::exec(),
    Commands::ListRemote => commands::list::exec_remote(),
    Commands::Uninstall { version } => commands::uninstall::exec(version),
    Commands::Use { version, local } => commands::use_version::exec(&mut meta, version, local),
    Commands::Alias { command } => commands::alias::exec(&mut meta, command),
    Commands::Activate => commands::activate::exec(&mut meta),
    Commands::Deactivate => commands::deactivate::exec(),
    Commands::Doctor => commands::doctor::exec(&mut meta),
    Commands::Upgrade { alias } => commands::upgrade::exec(&mut meta, alias),
    Commands::Exec {
      command: _,
      deno_version: _,
    } => {
      /* unused */
      Ok(())
    }
    Commands::Clean => commands::clean::exec(&mut meta),
    Commands::Registry { registry } => commands::registry::exec(&mut meta, registry),
  };

  if let Err(err) = result {
    eprintln!("\x1b[31merror:\x1b[39m: {}", err);
    std::process::exit(1);
  }
}
