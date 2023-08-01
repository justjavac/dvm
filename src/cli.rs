use std::env;

use clap::builder::PossibleValue;
use clap::{Parser, ValueEnum};
use clap_complete::Shell;
use clap_derive::{Parser, Subcommand};

use crate::commands;
use crate::consts::{
  AFTER_HELP, COMPLETIONS_HELP, REGISTRY_CN, REGISTRY_LIST_CN, REGISTRY_LIST_OFFICIAL, REGISTRY_NAME_CN,
  REGISTRY_NAME_OFFICIAL, REGISTRY_OFFICIAL,
};
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
      commands::exec::exec(meta, None, vec![]).unwrap();
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
  #[clap(long_about = COMPLETIONS_HELP)]
  Completions {
    #[arg(value_enum)]
    shell: Shell,
  },

  #[clap(about = "Show dvm info.")]
  Info,

  #[clap(about = "Install deno executable to the given version.")]
  #[clap(visible_aliases = & ["i", "add"])]
  #[clap(disable_version_flag = true)]
  Install {
    #[clap(long, help = "Only install to local, but not use")]
    no_use: bool,
    #[clap(help = "The version to install")]
    version: Option<String>,
  },

  #[clap(about = "List all installed versions")]
  #[clap(visible_aliases = & ["ls", "ll", "la"])]
  List,

  #[clap(about = "List all released versions")]
  #[clap(visible_aliases = & ["lr", "ls-remote"])]
  ListRemote,

  #[clap(about = "Uninstall a given version")]
  #[clap(visible_aliases = & ["un", "unlink", "rm", "remove"])]
  #[clap(disable_version_flag = true)]
  Uninstall {
    #[clap(help = "The version to uninstall")]
    version: Option<String>,
  },

  #[clap(about = "Use a given version or a semver range or a alias to the range.")]
  #[clap(disable_version_flag = true)]
  Use {
    #[clap(help = "The version, semver range or alias to use")]
    version: Option<String>,

    #[clap(
      short = 'L',
      long = "write-local",
      help = "Writing the version to the .dvmrc file of the current directory if present"
    )]
    write_local: bool,
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

  #[clap(about = "Upgrade aliases to the latest version, use `self` to upgrade dvm itself")]
  Upgrade {
    #[clap(help = "The alias to upgrade, use `self` to upgrade `dvm` itself, upgrade all aliases if not present")]
    alias: Option<String>,
  },

  #[clap(about = "Execute deno command with a specific deno version")]
  #[clap(disable_version_flag = true)]
  Exec {
    #[clap(help = "The command given to deno")]
    command: Option<String>,

    #[clap(help = "The version to use", long, short)]
    version: Option<String>,
  },

  #[clap(about = "Clean dvm cache")]
  Clean,

  #[clap(about = "Change registry that dvm fetch from")]
  Registry {
    #[clap(subcommand)]
    command: RegistryCommands,
  },

  #[clap(about = "Update remove version list local cache to the latest")]
  Update,
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

#[derive(Subcommand)]
pub enum RegistryCommands {
  #[clap(about = "List predefined registries")]
  List,

  #[clap(about = "Show current binary registry and version registry")]
  Show,

  #[clap(about = "Set registry to one of predefined registries")]
  Set {
    predefined: RegistryPredefined,

    #[clap(
      long = "write-local",
      short = 'L',
      help = "Write to current directory .dvmrc file instead of global(user-wide) config"
    )]
    write_local: bool,
  },

  #[clap(about = "Binary registry operations")]
  Binary {
    #[clap(subcommand)]
    sub: BinaryRegistryCommands,
  },

  #[clap(about = "Version registry operations")]
  Version {
    #[clap(subcommand)]
    sub: VersionRegistryCommands,
  },
}

#[derive(Subcommand)]
pub enum BinaryRegistryCommands {
  #[clap(about = "Show current binary registry")]
  Show,
  #[clap(about = "Set binary registry to one of predefined registries")]
  Set {
    custom: String,
    #[clap(
      long = "write-local",
      short = 'L',
      help = "Write to current directory .dvmrc file instead of global(user-wide) config"
    )]
    write_local: bool,
  },
}

#[derive(Subcommand)]
pub enum VersionRegistryCommands {
  #[clap(about = "Show current version registry")]
  Show,
  #[clap(about = "Set version registry to one of predefined registries")]
  Set {
    custom: String,
    #[clap(
      long = "write-local",
      short = 'L',
      help = "Write to current directory .dvmrc file instead of global(user-wide) config"
    )]
    write_local: bool,
  },
}

#[derive(Clone)]
pub enum RegistryPredefined {
  Official,
  CN,
}

impl ValueEnum for RegistryPredefined {
  fn value_variants<'a>() -> &'a [Self] {
    &[RegistryPredefined::Official, RegistryPredefined::CN]
  }

  fn from_str(input: &str, ignore_case: bool) -> Result<Self, String> {
    if (ignore_case && REGISTRY_NAME_OFFICIAL == input.to_ascii_lowercase()) || REGISTRY_NAME_OFFICIAL == input {
      Ok(RegistryPredefined::Official)
    } else if (ignore_case && REGISTRY_NAME_CN == input.to_ascii_lowercase()) || REGISTRY_NAME_CN == input {
      Ok(RegistryPredefined::CN)
    } else {
      Err(format!("{} is not a valid registry", input))
    }
  }

  fn to_possible_value(&self) -> Option<PossibleValue> {
    Some(PossibleValue::new(match self {
      RegistryPredefined::Official => REGISTRY_NAME_OFFICIAL,
      RegistryPredefined::CN => REGISTRY_NAME_CN,
    }))
  }
}

impl RegistryPredefined {
  pub fn get_version_url(&self) -> String {
    match self {
      RegistryPredefined::Official => REGISTRY_LIST_OFFICIAL,
      RegistryPredefined::CN => REGISTRY_LIST_CN,
    }
    .to_string()
  }

  pub fn get_binary_url(&self) -> String {
    match self {
      RegistryPredefined::Official => REGISTRY_OFFICIAL,
      RegistryPredefined::CN => REGISTRY_CN,
    }
    .to_string()
  }
}
