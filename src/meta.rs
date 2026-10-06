use crate::consts::DVM_CACHE_INVALID_TIMEOUT;
use crate::utils::{deno_version_path, dvm_root, dvm_versions, now};
use crate::version::VersionArg;
use colored::Colorize;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub const DEFAULT_ALIAS: phf::Map<&'static str, &'static str> = phf::phf_map! {
  "latest" => "*"
};

pub trait ToVersionReq {
  fn try_to_version_req(&self) -> anyhow::Result<VersionReq>;
}

#[derive(Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct VersionMapping {
  pub required: String,
  pub current: String,
}

impl VersionMapping {
  pub fn is_valid_mapping(&self) -> bool {
    let Ok(current) = Version::parse(&self.current) else {
      return false;
    };
    let Ok(req) = self.try_to_version_req() else {
      return false;
    };
    req.matches(&current)
  }
}

impl ToVersionReq for VersionMapping {
  fn try_to_version_req(&self) -> anyhow::Result<VersionReq> {
    VersionReq::from_str(&self.required).map_err(|err| anyhow::anyhow!(err))
  }
}

#[derive(Clone, Eq, PartialEq, Deserialize, Serialize, Debug)]
pub struct Alias {
  pub name: String,
  pub required: String,
}

impl ToVersionReq for Alias {
  fn try_to_version_req(&self) -> anyhow::Result<VersionReq> {
    VersionReq::from_str(&self.required).map_err(|err| anyhow::anyhow!(err))
  }
}

/// Legacy format used by older versions of dvm, where `versions` and `alias`
/// were stored as JSON arrays (Vec). We keep this around to support migration.
#[derive(Clone, Default, Eq, PartialEq, Deserialize)]
struct DvmMetaLegacy {
  pub versions: Vec<VersionMapping>,
  pub alias: Vec<Alias>,
}

#[derive(Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct DvmMeta {
  pub versions: HashMap<String, VersionMapping>,
  pub alias: HashMap<String, Alias>,
}

impl DvmMeta {
  pub fn path() -> PathBuf {
    let mut meta = dvm_root();
    meta.push(Path::new("dvm-metadata.json"));
    meta
  }

  pub fn new() -> Self {
    let path = DvmMeta::path();
    if path.exists() {
      let content = read_to_string(path);
      if let Ok(content) = content {
        // Try the new HashMap-based format first
        let config = serde_json::from_str::<DvmMeta>(content.as_str());
        if let Ok(mut config) = config {
          // Drop mappings that no longer point at an installed deno. An
          // unparseable `current` counts as gone rather than as a panic:
          // `DvmMeta::new` runs before every command, so panicking here would
          // also take down the `dvm doctor` / `dvm clean` meant to repair the
          // metadata.
          config
            .versions
            .retain(|_, mapping| Version::parse(&mapping.current).is_ok_and(|it| deno_version_path(&it).exists()));
          return config;
        }

        // Fall back to the legacy Vec-based format and migrate it
        let legacy = serde_json::from_str::<DvmMetaLegacy>(content.as_str());
        if let Ok(legacy) = legacy {
          let mut config = DvmMeta::from(legacy);
          config
            .versions
            .retain(|_, mapping| Version::parse(&mapping.current).is_ok_and(|it| deno_version_path(&it).exists()));
          // Best-effort: persist the migrated format so future reads are fast
          let _ = config.save();
          return config;
        }
      }
    }

    let config = DvmMeta::default();
    // Best-effort: the in-memory default is valid even if the disk write fails
    // (e.g. read-only home on a first run).
    let _ = config.save();
    config
  }

  pub fn clean_files(&self) {
    let cache_folder = dvm_versions();
    if let Ok(dir) = cache_folder.read_dir() {
      for entry in dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
          let Some(name) = path.file_name().and_then(|it| it.to_str()) else {
            continue;
          };

          // it's been pointed by dvm versions
          if self.versions.values().any(|it| it.current == name) {
            continue;
          }

          // it's not been outdated. An unreadable or garbled stub counts as
          // "no timestamp", i.e. outdated, instead of aborting the whole clean.
          let stub = path.join(".dvmstub");
          if stub.is_file() {
            let last_used = std::fs::read_to_string(&stub)
              .ok()
              .and_then(|content| content.trim().parse::<u128>().ok());
            if last_used.is_some_and(|it| it > now().saturating_sub(DVM_CACHE_INVALID_TIMEOUT)) {
              continue;
            }
          }

          println!("Cleaning version {}", name.bright_black());
          if let Err(err) = std::fs::remove_dir_all(&path) {
            eprintln!("Failed to clean version {}: {}", name, err);
          }
        }
      }
    }
  }

  ///
  /// set a version mapping
  ///   `required` is either a semver range or a alias to a semver rage
  ///   `current` is the current directory that the deno located in
  pub fn set_version_mapping(&mut self, required: String, current: String) -> anyhow::Result<()> {
    self.versions.insert(
      required.clone(),
      VersionMapping { required, current },
    );
    self.save()
  }

  /// Remove all version mappings whose `current` field matches the given version.
  /// Returns the number of mappings removed.
  pub fn remove_mappings_for_version(&mut self, version: &str) -> anyhow::Result<usize> {
    let before = self.versions.len();
    self.versions.retain(|_, it| it.current != version);
    let removed = before - self.versions.len();
    if removed > 0 {
      self.save()?;
    }
    Ok(removed)
  }

  ///
  /// get fold name of a given mapping,
  /// None if there haven't a deno version met the required semver range or alias that
  /// are installed already
  pub fn get_version_mapping(&self, required: &str) -> Option<String> {
    self.versions.get(required).map(|it| it.current.clone())
  }

  ///
  /// delete a version mapping
  /// this will also delete actual files.
  pub fn delete_version_mapping(&mut self, required: String) -> anyhow::Result<()> {
    self.versions.remove(&required);
    self.save()
  }

  ///
  /// list aliases
  /// including predefined aliases
  pub fn list_alias(&self) -> Vec<Alias> {
    let mut alias: Vec<Alias> = self.alias.values().cloned().collect();
    for (k, v) in DEFAULT_ALIAS.into_iter() {
      alias.insert(
        0,
        Alias {
          name: k.to_string(),
          required: v.to_string(),
        },
      );
    }

    alias
  }

  /// list version mappings
  pub fn list_version_mapping(&self) -> Vec<&VersionMapping> {
    self.versions.values().collect()
  }

  /// set a alias
  ///   name is alias name
  ///   required is a semver range
  pub fn set_alias(&mut self, name: String, required: String) -> anyhow::Result<()> {
    if DEFAULT_ALIAS.contains_key(name.as_str()) {
      return Ok(());
    }
    self.alias.insert(
      name.clone(),
      Alias { name, required },
    );
    self.save()
  }

  pub fn has_alias(&self, name: &str) -> bool {
    self.get_alias(name).is_some()
  }

  /// get the semver range of alias
  pub fn get_alias(&self, name: &str) -> Option<VersionArg> {
    if DEFAULT_ALIAS.contains_key(name) {
      VersionArg::from_str(DEFAULT_ALIAS[name]).ok()
    } else {
      self
        .alias
        .get(name)
        .and_then(|alias| VersionArg::from_str(&alias.required).ok())
    }
  }

  /// delete a alias
  pub fn delete_alias(&mut self, name: String) -> anyhow::Result<()> {
    self.alias.remove(&name);
    self.save()
  }

  pub fn resolve_version_req(&self, required: &str) -> anyhow::Result<VersionArg> {
    self
      .get_alias(required)
      .or_else(|| VersionArg::from_str(required).ok())
      .ok_or_else(|| anyhow::anyhow!("`{}` is not a valid semver version or alias", required))
  }

  /// write to disk (atomic)
  pub fn save(&self) -> anyhow::Result<()> {
    let file_path = DvmMeta::path();
    let json = serde_json::to_string_pretty(self)?;
    crate::utils::atomic_write(file_path, json.as_bytes())?;
    Ok(())
  }
}

impl From<DvmMetaLegacy> for DvmMeta {
  fn from(legacy: DvmMetaLegacy) -> Self {
    let versions = legacy
      .versions
      .into_iter()
      .map(|vm| (vm.required.clone(), vm))
      .collect();
    let alias = legacy
      .alias
      .into_iter()
      .map(|a| (a.name.clone(), a))
      .collect();
    DvmMeta { versions, alias }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn test_default_config() {
    let result = serde_json::to_string(&DvmMeta::default());
    assert!(result.is_ok());
    // HashMap serializes to a JSON object
    assert_eq!(result.unwrap(), "{\"versions\":{},\"alias\":{}}");
  }

  #[test]
  fn test_versions_config() {
    let mut conf = DvmMeta::default();
    conf.versions.insert(
      "~1.0.0".to_string(),
      VersionMapping {
        required: "~1.0.0".to_string(),
        current: "1.0.1".to_string(),
      },
    );
    let result = serde_json::to_string(&conf);
    assert!(result.is_ok());
    // HashMap order is not guaranteed, so we parse and check instead of string comparison
    let parsed: DvmMeta = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(parsed.versions.len(), 1);
    let vm = parsed.versions.get("~1.0.0").unwrap();
    assert_eq!(vm.required, "~1.0.0");
    assert_eq!(vm.current, "1.0.1");
  }

  #[test]
  fn test_alias_config() {
    let mut conf = DvmMeta::default();
    conf.alias.insert(
      "stable".to_string(),
      Alias {
        name: "stable".to_string(),
        required: "1.0.0".to_string(),
      },
    );
    conf.alias.insert(
      "two-point-o".to_string(),
      Alias {
        name: "two-point-o".to_string(),
        required: "2.0.0".to_string(),
      },
    );
    let result = serde_json::to_string(&conf);
    assert!(result.is_ok());
    let parsed: DvmMeta = serde_json::from_str(&result.unwrap()).unwrap();
    assert_eq!(parsed.alias.len(), 2);
    assert_eq!(parsed.alias.get("stable").unwrap().required, "1.0.0");
    assert_eq!(parsed.alias.get("two-point-o").unwrap().required, "2.0.0");
  }

  #[test]
  fn test_parse_valid() {
    let raw = json!(
        {
            "versions": {
                "~1.0.0": { "required": "~1.0.0", "current": "1.0.1" },
                "^1.0.0": { "required": "^1.0.0", "current": "1.2.0" }
            },
            "alias": {
                "latest": { "name": "latest", "required": "*" },
                "stable": { "name": "stable", "required": "^1.0.0" }
            }
        }
    );

    let parsed = DvmMeta::deserialize(raw);
    assert!(parsed.is_ok());
    let parsed = parsed.unwrap();
    assert_eq!(parsed.alias.len(), 2);
    assert_eq!(parsed.versions.len(), 2);

    let latest = parsed.alias.get("latest").unwrap();
    assert_eq!(latest.name, "latest");
    assert_eq!(latest.required, "*");
    assert_eq!(latest.try_to_version_req().unwrap(), VersionReq::parse("*").unwrap());
    assert!(latest.try_to_version_req().is_ok());

    let stable = parsed.alias.get("stable").unwrap();
    assert_eq!(stable.name, "stable");
    assert_eq!(stable.required, "^1.0.0");
    assert!(stable.try_to_version_req().is_ok());

    let v1 = parsed.versions.get("~1.0.0").unwrap();
    assert_eq!(v1.required, "~1.0.0");
    assert_eq!(v1.current, "1.0.1");
    assert!(v1.try_to_version_req().is_ok());
    assert!(v1.is_valid_mapping());

    let v2 = parsed.versions.get("^1.0.0").unwrap();
    assert_eq!(v2.required, "^1.0.0");
    assert_eq!(v2.current, "1.2.0");
    assert!(v2.try_to_version_req().is_ok());
    assert!(v2.is_valid_mapping());
  }

  #[test]
  fn test_migration_from_legacy_format() {
    // Simulate the old Vec-based JSON format
    let legacy_json = json!(
        {
            "versions": [
                { "required": "~1.0.0", "current": "1.0.1" },
                { "required": "^1.0.0", "current": "1.2.0" },
            ],
            "alias": [
                { "name": "stable", "required": "^1.0.0"},
                { "name": "lts", "required": "1.40.0" },
            ]
        }
    );

    let legacy: DvmMetaLegacy = serde_json::from_value(legacy_json).unwrap();
    let migrated = DvmMeta::from(legacy);

    assert_eq!(migrated.versions.len(), 2);
    assert_eq!(migrated.alias.len(), 2);
    assert_eq!(migrated.versions.get("~1.0.0").unwrap().current, "1.0.1");
    assert_eq!(migrated.versions.get("^1.0.0").unwrap().current, "1.2.0");
    assert_eq!(migrated.alias.get("stable").unwrap().required, "^1.0.0");
    assert_eq!(migrated.alias.get("lts").unwrap().required, "1.40.0");
  }

  #[test]
  fn test_set_and_get_version_mapping() {
    let mut meta = DvmMeta::default();
    meta.versions.insert(
      "~1.0.0".to_string(),
      VersionMapping {
        required: "~1.0.0".to_string(),
        current: "1.0.5".to_string(),
      },
    );
    assert_eq!(meta.get_version_mapping("~1.0.0"), Some("1.0.5".to_string()));
    assert_eq!(meta.get_version_mapping("^2.0.0"), None);
  }

  #[test]
  fn test_delete_version_mapping() {
    let mut meta = DvmMeta::default();
    meta.versions.insert(
      "~1.0.0".to_string(),
      VersionMapping {
        required: "~1.0.0".to_string(),
        current: "1.0.5".to_string(),
      },
    );
    assert!(meta.versions.contains_key("~1.0.0"));
    meta.versions.remove("~1.0.0");
    assert!(!meta.versions.contains_key("~1.0.0"));
  }

  #[test]
  fn test_remove_mappings_for_version() {
    let mut meta = DvmMeta::default();
    meta.versions.insert(
      "~1.0.0".to_string(),
      VersionMapping {
        required: "~1.0.0".to_string(),
        current: "1.0.5".to_string(),
      },
    );
    meta.versions.insert(
      "^1.0.0".to_string(),
      VersionMapping {
        required: "^1.0.0".to_string(),
        current: "1.0.5".to_string(),
      },
    );
    meta.versions.insert(
      "^2.0.0".to_string(),
      VersionMapping {
        required: "^2.0.0".to_string(),
        current: "2.0.0".to_string(),
      },
    );
    let before = meta.versions.len();
    meta.versions.retain(|_, it| it.current != "1.0.5");
    let removed = before - meta.versions.len();
    assert_eq!(removed, 2);
    assert_eq!(meta.versions.len(), 1);
    assert!(meta.versions.contains_key("^2.0.0"));
  }
}
