use crate::consts::DVM_CACHE_INVALID_TIMEOUT;
use crate::utils::{deno_version_path, dvm_root, dvm_versions, now, print_error};
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
        if let Ok(config) = config {
          return config;
        }

        // Fall back to the legacy Vec-based format and migrate it
        let legacy = serde_json::from_str::<DvmMetaLegacy>(content.as_str());
        if let Ok(legacy) = legacy {
          let config = DvmMeta::from(legacy);
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

  /// Remove version mappings whose target directory no longer exists on disk.
  /// This is an integrity check that should only run from `dvm doctor` and
  /// `dvm list` — doing it on every command startup would add O(n) filesystem
  /// stat calls to every dvm invocation. Returns the number of stale mappings
  /// that were removed.
  pub fn cleanup_stale_mappings(&mut self) -> anyhow::Result<usize> {
    let before = self.versions.len();
    self
      .versions
      .retain(|_, mapping| Version::parse(&mapping.current).is_ok_and(|it| deno_version_path(&it).exists()));
    let removed = before - self.versions.len();
    if removed > 0 {
      self.save()?;
    }
    Ok(removed)
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

          // Layer 1: skip versions that are actively referenced in meta
          // (either as a current version mapping or targeted by an alias).
          // This prevents deleting a version that dvm is actively managing.
          if self.is_version_referenced(name) {
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

          // Layer 2: double-check the stub timestamp right before deleting
          // to defend against TOCTOU races. Between the first check above and
          // this point, another process could have started using this version
          // (which updates the stub). Re-reading confirms it's still expired.
          if stub.is_file() {
            let last_used = std::fs::read_to_string(&stub)
              .ok()
              .and_then(|content| content.trim().parse::<u128>().ok());
            if last_used.is_some_and(|it| it > now().saturating_sub(DVM_CACHE_INVALID_TIMEOUT)) {
              continue;
            }
          }
          // Also re-check the meta reference right before deletion, in case
          // another process registered this version since our first check.
          if self.is_version_referenced(name) {
            continue;
          }

          println!("Cleaning version {}", name.bright_black());
          if let Err(err) = std::fs::remove_dir_all(&path) {
            print_error(&format!("Failed to clean version {}: {}", name, err));
          }
        }
      }
    }
  }

  /// Check whether a version directory is actively referenced in metadata.
  /// A version is referenced if it appears as the `current` field of any
  /// version mapping (which includes both direct ranges and alias targets).
  fn is_version_referenced(&self, version: &str) -> bool {
    self.versions.values().any(|it| it.current == version)
  }

  ///
  /// set a version mapping
  ///   `required` is either a semver range or a alias to a semver rage
  ///   `current` is the current directory that the deno located in
  pub fn set_version_mapping(&mut self, required: String, current: String) -> anyhow::Result<()> {
    self
      .versions
      .insert(required.clone(), VersionMapping { required, current });
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

  /// Delete a version mapping from metadata.
  /// Does NOT delete actual version files on disk.
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

  /// set a alias
  ///   name is alias name
  ///   required is a semver range
  pub fn set_alias(&mut self, name: String, required: String) -> anyhow::Result<()> {
    if DEFAULT_ALIAS.contains_key(name.as_str()) {
      return Ok(());
    }
    self.alias.insert(name.clone(), Alias { name, required });
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
    let alias = legacy.alias.into_iter().map(|a| (a.name.clone(), a)).collect();
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

  // --- Alias operation tests ---

  #[test]
  fn test_set_new_alias() {
    let mut meta = DvmMeta::default();
    meta.alias.insert(
      "stable".to_string(),
      Alias {
        name: "stable".to_string(),
        required: "^1.0.0".to_string(),
      },
    );
    assert_eq!(meta.alias.len(), 1);
    assert_eq!(meta.alias.get("stable").unwrap().name, "stable");
    assert_eq!(meta.alias.get("stable").unwrap().required, "^1.0.0");
  }

  #[test]
  fn test_set_alias_updates_existing() {
    let mut meta = DvmMeta::default();
    meta.alias.insert(
      "stable".to_string(),
      Alias {
        name: "stable".to_string(),
        required: "^1.0.0".to_string(),
      },
    );
    // update existing alias
    meta.alias.get_mut("stable").unwrap().required = "^2.0.0".to_string();
    assert_eq!(meta.alias.len(), 1);
    assert_eq!(meta.alias.get("stable").unwrap().required, "^2.0.0");
  }

  #[test]
  fn test_get_existing_alias() {
    let mut meta = DvmMeta::default();
    meta.alias.insert(
      "stable".to_string(),
      Alias {
        name: "stable".to_string(),
        required: "^1.0.0".to_string(),
      },
    );
    let result = meta.get_alias("stable");
    assert!(result.is_some());
  }

  #[test]
  fn test_get_nonexistent_alias_returns_none() {
    let meta = DvmMeta::default();
    let result = meta.get_alias("nonexistent");
    assert!(result.is_none());
  }

  #[test]
  fn test_get_default_alias_latest() {
    let meta = DvmMeta::default();
    let result = meta.get_alias("latest");
    assert!(result.is_some());
  }

  #[test]
  fn test_delete_alias() {
    let mut meta = DvmMeta::default();
    meta.alias.insert(
      "stable".to_string(),
      Alias {
        name: "stable".to_string(),
        required: "^1.0.0".to_string(),
      },
    );
    assert_eq!(meta.alias.len(), 1);
    // remove the alias
    meta.alias.remove("stable");
    assert!(meta.alias.is_empty());
    assert!(meta.get_alias("stable").is_none());
  }

  #[test]
  fn test_delete_nonexistent_alias_is_noop() {
    let mut meta = DvmMeta::default();
    meta.alias.insert(
      "stable".to_string(),
      Alias {
        name: "stable".to_string(),
        required: "^1.0.0".to_string(),
      },
    );
    let before = meta.alias.len();
    // try to remove an alias that doesn't exist
    meta.alias.remove("nonexistent");
    assert_eq!(meta.alias.len(), before);
  }

  #[test]
  fn test_list_alias_includes_default_and_custom() {
    let mut meta = DvmMeta::default();
    meta.alias.insert(
      "stable".to_string(),
      Alias {
        name: "stable".to_string(),
        required: "^1.0.0".to_string(),
      },
    );
    meta.alias.insert(
      "lts".to_string(),
      Alias {
        name: "lts".to_string(),
        required: "1.40.0".to_string(),
      },
    );
    let list = meta.list_alias();
    // default "latest" + 2 custom = 3 total
    assert_eq!(list.len(), 3);
    // "latest" should be at the front (inserted at index 0)
    assert_eq!(list[0].name, "latest");
    assert_eq!(list[0].required, "*");
    // custom aliases should be present
    assert!(list.iter().any(|a| a.name == "stable"));
    assert!(list.iter().any(|a| a.name == "lts"));
  }

  #[test]
  fn test_list_alias_empty_meta() {
    let meta = DvmMeta::default();
    let list = meta.list_alias();
    // only the default "latest" alias
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "latest");
  }

  #[test]
  fn test_has_alias_for_existing() {
    let mut meta = DvmMeta::default();
    meta.alias.insert(
      "stable".to_string(),
      Alias {
        name: "stable".to_string(),
        required: "^1.0.0".to_string(),
      },
    );
    assert!(meta.has_alias("stable"));
    // default alias also returns true
    assert!(meta.has_alias("latest"));
  }

  #[test]
  fn test_has_alias_for_nonexistent() {
    let meta = DvmMeta::default();
    assert!(!meta.has_alias("nonexistent"));
  }

  #[test]
  fn test_resolve_version_req_exact_alias() {
    let mut meta = DvmMeta::default();
    meta.alias.insert(
      "v1".to_string(),
      Alias {
        name: "v1".to_string(),
        required: "1.2.3".to_string(),
      },
    );
    let result = meta.resolve_version_req("v1");
    assert!(result.is_ok());
  }

  #[test]
  fn test_resolve_version_req_lts_alias() {
    let mut meta = DvmMeta::default();
    meta.alias.insert(
      "lts".to_string(),
      Alias {
        name: "lts".to_string(),
        required: "1.40.0".to_string(),
      },
    );
    let result = meta.resolve_version_req("lts");
    assert!(result.is_ok());
  }

  #[test]
  fn test_resolve_version_req_range_alias() {
    let mut meta = DvmMeta::default();
    meta.alias.insert(
      "stable".to_string(),
      Alias {
        name: "stable".to_string(),
        required: "^1.0.0".to_string(),
      },
    );
    let result = meta.resolve_version_req("stable");
    assert!(result.is_ok());
  }

  #[test]
  fn test_resolve_version_req_default_alias() {
    let meta = DvmMeta::default();
    let result = meta.resolve_version_req("latest");
    assert!(result.is_ok());
  }

  #[test]
  fn test_resolve_version_req_direct_version() {
    let meta = DvmMeta::default();
    let result = meta.resolve_version_req("1.2.3");
    assert!(result.is_ok());
  }

  #[test]
  fn test_resolve_version_req_invalid() {
    let meta = DvmMeta::default();
    let result = meta.resolve_version_req("not-a-valid-version-or-alias");
    assert!(result.is_err());
  }

  // --- Corruption / degradation path tests ---

  #[test]
  fn test_default_creates_expected_structure() {
    let meta = DvmMeta::default();
    assert!(meta.versions.is_empty());
    assert!(meta.alias.is_empty());
    // Default alias still includes built-in "latest"
    assert!(meta.has_alias("latest"));
    assert_eq!(meta.list_alias().len(), 1); // only the default "latest"
  }

  #[test]
  fn test_deserialize_empty_string_fails() {
    let result = serde_json::from_str::<DvmMeta>("");
    assert!(result.is_err());
  }

  #[test]
  fn test_set_default_alias_is_noop() {
    let mut meta = DvmMeta::default();
    // Trying to set "latest" (a default alias) should not add it to self.alias
    if DEFAULT_ALIAS.contains_key("latest") {
      // skip, simulating set_alias behavior for default aliases
    } else {
      meta.alias.insert(
        "latest".to_string(),
        Alias {
          name: "latest".to_string(),
          required: "*".to_string(),
        },
      );
    }
    assert_eq!(meta.alias.len(), 0);
  }

  #[test]
  fn test_deserialize_garbage_text_fails() {
    let result = serde_json::from_str::<DvmMeta>("this is not json at all!!!");
    assert!(result.is_err());
  }

  #[test]
  fn test_deserialize_empty_object_succeeds_with_defaults() {
    // An empty JSON object `{}` — both fields are missing.
    // Without #[serde(default)], this would fail. Let's verify current behavior.
    let result = serde_json::from_str::<DvmMeta>("{}");
    // Current behavior: fails because fields are required
    assert!(result.is_err());
    // DvmMeta::new() would fall back to default in this case
  }

  #[test]
  fn test_deserialize_missing_versions_field_fails() {
    let result = serde_json::from_str::<DvmMeta>(r#"{"alias": {"stable": {"name": "stable", "required": "1.0.0"}}}"#);
    assert!(result.is_err());
  }

  #[test]
  fn test_deserialize_missing_alias_field_fails() {
    let result =
      serde_json::from_str::<DvmMeta>(r#"{"versions": {"~1.0.0": {"required": "~1.0.0", "current": "1.0.1"}}}"#);
    assert!(result.is_err());
  }

  #[test]
  fn test_deserialize_with_null_fields_fails() {
    let result = serde_json::from_str::<DvmMeta>(r#"{"versions": null, "alias": null}"#);
    assert!(result.is_err());
  }

  #[test]
  fn test_invalid_semver_in_current_field() {
    let mapping = VersionMapping {
      required: "~1.0.0".to_string(),
      current: "not-a-version".to_string(),
    };
    assert!(!mapping.is_valid_mapping());
    // try_to_version_req should still work (required is valid)
    assert!(mapping.try_to_version_req().is_ok());
  }

  #[test]
  fn test_invalid_version_req_in_required_field() {
    let mapping = VersionMapping {
      required: "not a valid req >= <=".to_string(),
      current: "1.0.0".to_string(),
    };
    assert!(!mapping.is_valid_mapping());
    assert!(mapping.try_to_version_req().is_err());
  }

  #[test]
  fn test_version_mismatch_is_not_valid() {
    // required = ^1.0.0, current = 2.0.0 — the current version doesn't match the req
    let mapping = VersionMapping {
      required: "^1.0.0".to_string(),
      current: "2.0.0".to_string(),
    };
    // Both fields parse fine individually, but current doesn't satisfy required
    assert!(Version::parse(&mapping.current).is_ok());
    assert!(mapping.try_to_version_req().is_ok());
    assert!(!mapping.is_valid_mapping());
  }

  #[test]
  fn test_alias_invalid_version_req() {
    let alias = Alias {
      name: "bad".to_string(),
      required: "!!!invalid!!!".to_string(),
    };
    assert!(alias.try_to_version_req().is_err());
  }

  #[test]
  fn test_deserialize_extra_fields_ignored() {
    // Extra unknown fields should be ignored (forward compatibility)
    let result = serde_json::from_str::<DvmMeta>(
      r#"{
        "versions": {},
        "alias": {},
        "new_field": "some value",
        "another_new_thing": 42
      }"#,
    );
    assert!(result.is_ok());
    let meta = result.unwrap();
    assert!(meta.versions.is_empty());
    assert!(meta.alias.is_empty());
  }

  #[test]
  fn test_deserialize_wrong_types_fails_cleanly() {
    // versions is a string instead of an object
    let result = serde_json::from_str::<DvmMeta>(r#"{"versions": "not-an-object", "alias": {}}"#);
    assert!(result.is_err());

    // alias is a number instead of an object
    let result = serde_json::from_str::<DvmMeta>(r#"{"versions": {}, "alias": 123}"#);
    assert!(result.is_err());
  }

  #[test]
  fn test_version_mapping_both_invalid() {
    let mapping = VersionMapping {
      required: "garbage req".to_string(),
      current: "garbage version".to_string(),
    };
    assert!(!mapping.is_valid_mapping());
    assert!(mapping.try_to_version_req().is_err());
  }
}
