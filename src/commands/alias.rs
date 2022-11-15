use crate::cli::AliasCommands;
use crate::version::{find_max_matching_version, local_versions, remote_versions_or_cached, version_req_parse};
use crate::{DvmMeta, DEFAULT_ALIAS};

use anyhow::Result;
use colored::{ColoredString, Colorize};
use phf::phf_map;

const ALIAS_COLORS: phf::Map<&str, (u8, u8, u8)> = phf_map! {
    "lighter" => (0xD1, 0xFA, 0xFF),        // unused
    "norm" => (0x9B, 0xD1, 0xE5),           // user add alias
    "darker" => (0x6A, 0x8E, 0xAE),         // default alias
    "highlight" => (0x15, 0x71, 0x45),      // latest version
    "highlightdark" => (0x57, 0xA7, 0x73),  // need upgrade
};

fn apply_alias_color(a: &str, c: &str) -> ColoredString {
  a.truecolor(
    ALIAS_COLORS.get(c).unwrap().0,
    ALIAS_COLORS.get(c).unwrap().1,
    ALIAS_COLORS.get(c).unwrap().2,
  )
}

pub fn exec(meta: &mut DvmMeta, command: AliasCommands) -> Result<()> {
  match command {
    AliasCommands::Set { name, content } => {
      version_req_parse(content.as_str());
      meta.set_alias(name, content);
      Ok(())
    }
    AliasCommands::Unset { name } => {
      meta.delete_alias(name);
      Ok(())
    }
    AliasCommands::List => {
      let remote_versions = remote_versions_or_cached().unwrap();
      let local_versions = local_versions();
      let get_upgrade_version = |version_str: &str| {
        let max_remote_version =
          find_max_matching_version(version_str, remote_versions.iter().map(AsRef::as_ref)).unwrap();

        let max_local_version =
          find_max_matching_version(version_str, local_versions.iter().map(AsRef::as_ref)).unwrap();
        if let (Some(max_remote), Some(max_local)) = (max_remote_version, max_local_version) {
          if max_remote > max_local {
            return Some(max_remote);
          }
        }
        None
      };
      for (key, val) in DEFAULT_ALIAS.entries() {
        let upgrade_version = get_upgrade_version(val);
        if let Some(upgrade_version) = upgrade_version {
          println!(
            "{} -> {} ( -> {})",
            apply_alias_color(key, "norm"),
            val,
            apply_alias_color(upgrade_version.to_string().as_str(), "highlightdark")
          );
          continue;
        }
        println!("{} -> {}", apply_alias_color(key, "darker"), val);
      }
      for alias in &meta.alias {
        let upgrade_version = get_upgrade_version(&alias.required);
        if let Some(upgrade_version) = upgrade_version {
          println!(
            "{} -> {} ( -> {})",
            apply_alias_color(&alias.name, "norm"),
            &alias.required,
            apply_alias_color(upgrade_version.to_string().as_str(), "highlightdark")
          );
          continue;
        }
        println!("{} -> {}", apply_alias_color(&alias.name, "norm"), alias.required);
      }
      Ok(())
    }
  }
}
