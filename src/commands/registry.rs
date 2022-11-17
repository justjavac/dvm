use std::process;

use crate::cli::{BinaryRegistryCommands, RegistryCommands, VersionRegistryCommands};
use crate::consts::REGISTRY_NAME_OFFICIAL;
use crate::consts::REGISTRY_OFFICIAL;
use crate::consts::{DVM_CONFIGRC_KEY_REGISTRY_BINARY, DVM_CONFIGRC_KEY_REGISTRY_VERSION, REGISTRY_NAME_CN};
use crate::consts::{REGISTRY_CN, REGISTRY_LIST_CN, REGISTRY_LIST_OFFICIAL};
use crate::DvmMeta;

use crate::configrc::{rc_get, rc_update};
use crate::utils::is_http_like_url;
use anyhow::Result;
use colored::Colorize;

pub fn exec(meta: &mut DvmMeta, registry: RegistryCommands) -> Result<()> {
  let rc_binary_registry = rc_get(DVM_CONFIGRC_KEY_REGISTRY_BINARY).unwrap_or_else(|_| REGISTRY_OFFICIAL.to_string());
  let rc_version_registry = rc_get(DVM_CONFIGRC_KEY_REGISTRY_VERSION).unwrap_or_else(|_| REGISTRY_OFFICIAL.to_string());

  match registry {
    RegistryCommands::List => {
      println!("{}:", "official".bright_blue());
      println!("  binary_registry\t{}", REGISTRY_OFFICIAL);
      println!("  version_registry\t{}", REGISTRY_LIST_OFFICIAL);

      println!("{}:", "cn".bright_blue());
      println!("  binary_registry\t{}", REGISTRY_CN);
      println!("  version_registry\t{}", REGISTRY_LIST_CN);
      println!("Use {} to set the registry.", "dvm registry set <name>".bright_green());
      println!("for example: {}", "dvm registry set official".bright_green());
    }
    RegistryCommands::Show => {
      println! {"{}: ", "current registry info".bright_blue()};
      println!("  binary_registry\t{}", rc_binary_registry);
      println!("  version_registry\t{}", rc_version_registry);
    }
    RegistryCommands::Set {
      predefined,
      write_local,
    } => {
      rc_update(
        write_local,
        DVM_CONFIGRC_KEY_REGISTRY_BINARY,
        &predefined.get_binary_url(),
      )?;
      rc_update(
        write_local,
        DVM_CONFIGRC_KEY_REGISTRY_VERSION,
        &predefined.get_version_url(),
      )?;
    }
    RegistryCommands::Binary { sub } => match sub {
      BinaryRegistryCommands::Show => {
        println!("{}: {}", "current binary registry".bright_blue(), rc_binary_registry);
      }
      BinaryRegistryCommands::Set { custom, write_local } => {
        if custom == REGISTRY_NAME_OFFICIAL {
          rc_update(write_local, DVM_CONFIGRC_KEY_REGISTRY_BINARY, REGISTRY_OFFICIAL)?;
        } else if custom == REGISTRY_NAME_CN {
          rc_update(write_local, DVM_CONFIGRC_KEY_REGISTRY_BINARY, REGISTRY_CN)?;
        } else if is_http_like_url(&custom) {
          rc_update(write_local, DVM_CONFIGRC_KEY_REGISTRY_BINARY, &custom)?;
        } else {
          println!("{}: {}", "invalid registry".bright_red(), custom);
          process::exit(1);
        }
      }
    },
    RegistryCommands::Version { sub } => match sub {
      VersionRegistryCommands::Show => {
        println!("{}: {}", "current version registry".bright_blue(), rc_version_registry);
      }
      VersionRegistryCommands::Set { custom, write_local } => {
        if custom == REGISTRY_NAME_OFFICIAL {
          rc_update(write_local, DVM_CONFIGRC_KEY_REGISTRY_VERSION, REGISTRY_LIST_OFFICIAL)?;
        } else if custom == REGISTRY_NAME_CN {
          rc_update(write_local, DVM_CONFIGRC_KEY_REGISTRY_VERSION, REGISTRY_LIST_CN)?;
        } else if is_http_like_url(&custom) {
          rc_update(write_local, DVM_CONFIGRC_KEY_REGISTRY_VERSION, &custom)?;
        } else {
          eprintln!("The {} is not valid URL, please starts with `http` or `https`", custom);
          eprintln!("Registry will not be changed");
          process::exit(1)
        }
      }
    },
  };

  meta.save();
  Ok(())
}
