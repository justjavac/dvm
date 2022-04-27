extern crate core;

mod commands;
mod consts;
mod meta;
mod utils;
pub mod version;

use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use clap_derive::{Parser, Subcommand};
use meta::DvmMeta;

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
  },
}

pub fn main() {
  let cli = Cli::parse();
  let mut meta = DvmMeta::new();

  let result = match cli.command {
    Commands::Completions { shell } => commands::completions::exec(&mut Cli::command(), shell),
    Commands::Info => commands::info::exec(),
    Commands::Install { no_use, version } => commands::install::exec(no_use, version),
    Commands::List => commands::list::exec(),
    Commands::ListRemote => commands::list::exec_remote(),
    Commands::Uninstall { version } => commands::uninstall::exec(version),
    Commands::Use { version } => commands::use_version::exec(&mut meta, version),
  };

  if let Err(err) = result {
    eprintln!("\x1b[31merror:\x1b[39m: {}", err);
    std::process::exit(1);
  }
}
