use crate::{AliasCommands, DvmMeta, DEFAULT_ALIAS};
use anyhow::Result;
use colored::{ColoredString, Colorize};
use phf::phf_map;
use semver::VersionReq;

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
      VersionReq::parse(content.as_str()).unwrap_or_else(|_| panic!("unexpected alias content: {}", content));
      meta.set_alias(name, content);
      Ok(())
    }
    AliasCommands::Unset { name } => {
      meta.delete_alias(name);
      Ok(())
    }
    AliasCommands::List => {
      for (key, val) in DEFAULT_ALIAS.entries() {
        println!("{} -> {}", apply_alias_color(key, "darker"), val);
      }

      for alias in &meta.alias {
        println!("{} -> {}", apply_alias_color(&alias.name, "norm"), alias.required);
      }
      Ok(())
    }
  }
}
