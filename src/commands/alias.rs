use crate::{
  version::{local_versions, remote_versions},
  AliasCommands, DvmMeta, DEFAULT_ALIAS,
};
use anyhow::Result;
use colored::{ColoredString, Colorize};
use phf::phf_map;
use semver::{Version, VersionReq};

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
      let remote_versions = remote_versions();
      let local_versions = local_versions();
      for alias in &meta.alias {
        if let Ok(ref remote_versions) = remote_versions {
          let max_remote_version = find_max_matching_version(
            &alias.required.as_str(),
            remote_versions.iter().map(|string| string.as_str()),
          ).unwrap();

          let max_local_version = find_max_matching_version(
            &alias.required.as_str(),
            local_versions.iter().map(|string| string.as_str()),
          ).unwrap();

          if let (Some(max_remote), Some(max_local)) = (max_remote_version, max_local_version) {
            if max_remote > max_local {
              println!(
                "{} -> {} ( -> {})",
                apply_alias_color(&alias.name, "norm"),
                &alias.required,
                apply_alias_color(max_remote.to_string().as_str(), "highlightdark")
              );
              continue;
            }
          }
        }
        println!("{} -> {}", apply_alias_color(&alias.name, "norm"), alias.required);
      }
      Ok(())
    }
  }
}
fn find_max_matching_version<'a, I>(version_req_str: &str, iterable: I) -> Result<Option<Version>>
where
  I: IntoIterator<Item = &'a str>,
{
  let version_req =
    VersionReq::parse(version_req_str).expect(format!("unexpected version semver: {}", version_req_str).as_str());
  Ok(
    iterable
      .into_iter()
      .filter_map(|s| Version::parse(s).ok())
      .filter(|s| version_req.matches(s))
      .max(),
  )
}
