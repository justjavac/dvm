use std::process;

use crate::cli::{BinaryRegistryCommands, RegistryCommands, VersionRegistryCommands};
use crate::consts::{REGISTRY_CN, REGISTRY_LIST_CN, REGISTRY_LIST_OFFICIAL};
use crate::consts::REGISTRY_NAME_CN;
use crate::consts::REGISTRY_NAME_OFFICIAL;
use crate::consts::REGISTRY_OFFICIAL;
use crate::DvmMeta;

use anyhow::Result;
use colored::Colorize;
use crate::utils::is_http_like_url;

pub fn exec(meta: &mut DvmMeta, registry: RegistryCommands) -> Result<()> {
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
            println!("  binary_registry\t{}", meta.registry.binary);
            println!("  version_registry\t{}", meta.registry.version);
        }
        RegistryCommands::Set { predefined } => {
            meta.registry.binary = predefined.get_binary_url();
            meta.registry.version = predefined.get_version_url();
        }
        RegistryCommands::Binary { sub } => {
            match sub {
                BinaryRegistryCommands::Show => {
                    println!("{}: {}", "current binary registry".bright_blue(), meta.registry.binary);
                }
                BinaryRegistryCommands::Set { custom } => {
                    if custom == REGISTRY_NAME_OFFICIAL {
                        meta.registry.binary = REGISTRY_OFFICIAL.to_string();
                    } else if custom == REGISTRY_NAME_CN {
                        meta.registry.binary = REGISTRY_CN.to_string();
                    } else if is_http_like_url(&custom) {
                        meta.registry.binary = custom;
                    } else {
                        println!("{}: {}", "invalid registry".bright_red(), custom);
                        process::exit(1);
                    }
                }
            }
        }
        RegistryCommands::Version { sub } => {
            match sub {
                VersionRegistryCommands::Show => {
                    println!("{}: {}", "current version registry".bright_blue(), meta.registry.version);
                }
                VersionRegistryCommands::Set { custom } => {
                    if custom == REGISTRY_NAME_OFFICIAL {
                        meta.registry.version = REGISTRY_LIST_OFFICIAL.to_string();
                    } else if custom == REGISTRY_NAME_CN {
                        meta.registry.version = REGISTRY_LIST_CN.to_string();
                    } else if is_http_like_url(&custom) {
                        meta.registry.version = custom;
                    } else {
                        eprintln!(
                            "The {} is not valid URL, please starts with `http` or `https`",
                            custom
                        );
                        eprintln!("Registry will not be changed");
                        process::exit(1)
                    }
                }
            }
        }
    };

    meta.save();
    Ok(())
}

