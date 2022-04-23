mod commands;
mod utils;
pub mod version;
use clap::{AppSettings, IntoApp, Parser};
use clap_complete::Shell;
use clap_derive::{Parser, Subcommand};
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
  
\x1b[33mNOTE:\x1b[39m
  To remove, delete, or uninstall dvm - just remove the \x1b[36m`$DVM_DIR`\x1b[39m folder (usually \x1b[36m`~/.dvm`\x1b[39m)";

static COMPLETIONS_HELP: &str = "Output shell completion script to standard output.
  \x1b[35m
  dvm completions bash > /usr/local/etc/bash_completion.d/dvm.bash
  source /usr/local/etc/bash_completion.d/dvm.bash\x1b[39m";

#[derive(Parser)]
#[clap(version, about)]
#[clap(after_help = AFTER_HELP)]
#[clap(global_setting(AppSettings::PropagateVersion))]
#[clap(global_setting(AppSettings::UseLongFormatForHelpSubcommand))]
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

  #[clap(about = "Switch to a given version which is locally installed, it's guaranteed to not access the internet")]
  Switch {
    #[clap(help = "The version to switch")]
    version: String,
  },

  #[clap(
    about = "Use a given version or a special tag, automatically install when the given version is not installed. If the version you gave is `latest`, dvm will always tracking the latest version and automatically upgrade deno to the latest version every time you run deno (not implemented yet)"
  )]
  Use {
    #[clap(help = "The version to use, or a special tag `latest`")]
    version: String,
  },
}

pub fn main() {
  let cli = Cli::parse();

  let result = match cli.command {
    Commands::Completions { shell } => commands::completions::exec(&mut Cli::into_app(), shell),
    Commands::Info => commands::info::exec(),
    Commands::Install { no_use, version } => commands::install::exec(no_use, version),
    Commands::List => commands::list::exec(),
    Commands::ListRemote => commands::list::exec_remote(),
    Commands::Uninstall { version } => commands::uninstall::exec(version),
    Commands::Switch { version } => commands::switch::exec(version),
    Commands::Use { version } => commands::r#use::exec(version),
  };

  if let Err(err) = result {
    eprintln!("\x1b[31merror:\x1b[39m: {}", err);
    std::process::exit(1);
  }
}
