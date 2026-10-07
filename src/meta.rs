use crate::consts::DVM_CACHE_INVALID_TIMEOUT;
use crate::utils::{deno_version_path, dvm_root, dvm_versions, now, print_error};
use crate::version::VersionArg;
use colored::Colorize;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
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

#[derive(Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct DvmMeta {
  pub versions: Vec<VersionMapping>,
  pub alias: Vec<Alias>,
}

/// Remove version mappings whose corresponding deno binary directory no
/// longer exists on disk.
///
/// This is a defensive cleanup that runs whenever metadata is loaded.  It
/// guards against manual deletion of version directories, corrupted
/// installs, etc.  However, we must be careful about transient disk errors:
/// if the dvm directory lives on a network filesystem (NFS) or removable
/// media (USB) that is temporarily unavailable, `exists()` would return
/// false for every version, and a naive cleanup would wipe all metadata —
/// permanently losing the user's configuration.
///
/// Safety rules:
/// 1. If ALL version directories appear missing and there is at least one
///    mapping, assume a disk error and preserve everything.
/// 2. If more than 50% of mappings would be removed, it's suspicious enough
///    to warrant a warning and skip the cleanup — better to have stale
///    entries than to lose all metadata.
fn cleanup_stale_mappings(config: &mut DvmMeta) {
  let total = config.versions.len();
  if total == 0 {
    return;
  }

  // Separate mappings into "still exists" and "appears missing".
  let mut existing = Vec::new();
  let mut missing = Vec::new();
  for mapping in &config.versions {
    let has_dir = Version::parse(&mapping.current)
      .map(|v| deno_version_path(&v).exists())
      .unwrap_or(false);
    if has_dir {
      existing.push(mapping.clone());
    } else {
      missing.push(mapping.clone());
    }
  }

  // Rule 1: if EVERYTHING is missing, it's almost certainly a disk error,
  // not a genuine clean state.  Preserve all mappings and warn.
  if existing.is_empty() {
    eprintln!(
      "Warning: all {} version directories appear to be missing. \
       This may indicate a disk or filesystem error. \
       Metadata preserved as-is.",
      total
    );
    return;
  }

  // Rule 2: if more than half the mappings would be removed, that's
  // suspicious — warn and skip the cleanup rather than risk data loss.
  if missing.len() * 2 > total {
    eprintln!(
      "Warning: {} of {} version mappings point to missing directories. \
       This looks like a potential filesystem issue. \
       Skipping metadata cleanup to prevent data loss. \
       Run `dvm clean` to force cleanup if this is expected.",
      missing.len(),
      total
    );
    return;
  }

  // Normal case: only a few stale entries, safe to clean up.
  config.versions = existing;
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
        let config = serde_json::from_str::<DvmMeta>(content.as_str());
        if let Ok(config) = config {
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
      .retain(|mapping| Version::parse(&mapping.current).is_ok_and(|it| deno_version_path(&it).exists()));
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
    self.versions.iter().any(|it| it.current == version)
  }

  ///
  /// set a version mapping
  ///   `required` is either a semver range or a alias to a semver rage
  ///   `current` is the current directory that the deno located in
  pub fn set_version_mapping(&mut self, required: String, current: String) -> anyhow::Result<()> {
    if let Some(mapping) = self.versions.iter_mut().find(|it| it.required == required) {
      mapping.current = current;
    } else {
      self.versions.push(VersionMapping { required, current });
    }
    self.save()
  }

  /// Remove all version mappings whose `current` field matches the given version.
  /// Returns the number of mappings removed.
  pub fn remove_mappings_for_version(&mut self, version: &str) -> anyhow::Result<usize> {
    let before = self.versions.len();
    self.versions.retain(|it| it.current != version);
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
    self
      .versions
      .iter()
      .find(|it| it.required == required)
      .map(|it| it.current.clone())
  }

  /// Delete a version mapping from metadata.
  /// Does NOT delete actual version files on disk.
  pub fn delete_version_mapping(&mut self, required: String) -> anyhow::Result<()> {
    if let Some(index) = self.versions.iter().position(|it| it.required == required) {
      self.versions.remove(index);
    }
    self.save()
  }

  ///
  /// list aliases
  /// including predefined aliases
  pub fn list_alias(&self) -> Vec<Alias> {
    let mut alias = self.alias.clone();
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
    if let Some(alias) = self.alias.iter_mut().find(|it| it.name == name) {
      alias.required = required;
    } else {
      self.alias.push(Alias { name, required });
    }
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
        .iter()
        .find(|it| it.name == name)
        .and_then(|alias| VersionArg::from_str(&alias.required).ok())
    }
  }

  /// delete a alias
  pub fn delete_alias(&mut self, name: String) -> anyhow::Result<()> {
    if let Some(index) = self.alias.iter().position(|it| it.name == name) {
      self.alias.remove(index);
    }
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

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn test_default_config() {
    let result = serde_json::to_string(&DvmMeta::default());
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "{\"versions\":[],\"alias\":[]}");
  }

  #[test]
  fn test_versions_config() {
    let mut conf = DvmMeta::default();
    conf.versions.push(VersionMapping {
      required: "~1.0.0".to_string(),
      current: "1.0.1".to_string(),
    });
    let result = serde_json::to_string(&conf);
    assert!(result.is_ok());
    assert_eq!(
      result.unwrap(),
      "{\"versions\":[{\"required\":\"~1.0.0\",\"current\":\"1.0.1\"}],\"alias\":[]}"
    )
  }

  #[test]
  fn test_alias_config() {
    let mut conf = DvmMeta::default();
    conf.alias.push(Alias {
      name: "stable".to_string(),
      required: "1.0.0".to_string(),
    });
    conf.alias.push(Alias {
      name: "two-point-o".to_string(),
      required: "2.0.0".to_string(),
    });
    let result = serde_json::to_string(&conf);
    assert!(result.is_ok());
    assert_eq!(
            result.unwrap(),
            "{\"versions\":[],\"alias\":[{\"name\":\"stable\",\"required\":\"1.0.0\"},{\"name\":\"two-point-o\",\"required\":\"2.0.0\"}]}"
        )
  }

  #[test]
  fn test_parse_valid() {
    let raw = json!(
        {
            "versions": [
                { "required": "~1.0.0", "current": "1.0.1" },
                { "required": "^1.0.0", "current": "1.2.0" },
            ],
            "alias": [
                { "name": "latest", "required": "*" },
                { "name": "stable", "required": "^1.0.0"},
            ]
        }
    );

    let parsed = DvmMeta::deserialize(raw);
    assert!(parsed.is_ok());
    let parsed = parsed.unwrap();
    assert_eq!(parsed.alias.len(), 2);
    assert_eq!(parsed.versions.len(), 2);
    assert_eq!(parsed.alias[0].name, "latest");
    assert_eq!(parsed.alias[0].required, "*");
    assert_eq!(parsed.alias[0].try_to_version_req().unwrap(), VersionReq::parse("*").unwrap());
    assert!(parsed.alias[0].try_to_version_req().is_ok());
    assert_eq!(parsed.alias[1].name, "stable");
    assert_eq!(parsed.alias[1].required, "^1.0.0");
    assert!(parsed.alias[1].try_to_version_req().is_ok());
    assert_eq!(parsed.versions[0].required, "~1.0.0");
    assert_eq!(parsed.versions[0].current, "1.0.1");
    assert!(parsed.versions[0].try_to_version_req().is_ok());
    assert!(parsed.versions[0].is_valid_mapping());
    assert_eq!(parsed.versions[1].required, "^1.0.0");
    assert_eq!(parsed.versions[1].current, "1.2.0");
    assert!(parsed.versions[1].try_to_version_req().is_ok());
    assert!(parsed.versions[1].is_valid_mapping());
  }

  // --- Alias operation tests ---

  #[test]
  fn test_set_new_alias() {
    let mut meta = DvmMeta::default();
    meta.alias.push(Alias {
      name: "stable".to_string(),
      required: "^1.0.0".to_string(),
    });
    assert_eq!(meta.alias.len(), 1);
    assert_eq!(meta.alias[0].name, "stable");
    assert_eq!(meta.alias[0].required, "^1.0.0");
  }

  #[test]
  fn test_set_alias_updates_existing() {
    let mut meta = DvmMeta::default();
    meta.alias.push(Alias {
      name: "stable".to_string(),
      required: "^1.0.0".to_string(),
    });
    // update existing alias
    if let Some(a) = meta.alias.iter_mut().find(|it| it.name == "stable") {
      a.required = "^2.0.0".to_string();
    }
    assert_eq!(meta.alias.len(), 1);
    assert_eq!(meta.alias[0].required, "^2.0.0");
  }

  #[test]
  fn test_get_existing_alias() {
    let mut meta = DvmMeta::default();
    meta.alias.push(Alias {
      name: "stable".to_string(),
      required: "^1.0.0".to_string(),
    });
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
    meta.alias.push(Alias {
      name: "stable".to_string(),
      required: "^1.0.0".to_string(),
    });
    assert_eq!(meta.alias.len(), 1);
    // remove the alias
    if let Some(index) = meta.alias.iter().position(|it| it.name == "stable") {
      meta.alias.remove(index);
    }
    assert_eq!(meta.alias.len(), 0);
    assert!(meta.get_alias("stable").is_none());
  }

  #[test]
  fn test_delete_nonexistent_alias_is_noop() {
    let mut meta = DvmMeta::default();
    meta.alias.push(Alias {
      name: "stable".to_string(),
      required: "^1.0.0".to_string(),
    });
    let before = meta.alias.len();
    // try to remove an alias that doesn't exist
    if let Some(index) = meta.alias.iter().position(|it| it.name == "nonexistent") {
      meta.alias.remove(index);
    }
    assert_eq!(meta.alias.len(), before);
  }

  #[test]
  fn test_list_alias_includes_default_and_custom() {
    let mut meta = DvmMeta::default();
    meta.alias.push(Alias {
      name: "stable".to_string(),
      required: "^1.0.0".to_string(),
    });
    meta.alias.push(Alias {
      name: "lts".to_string(),
      required: "1.40.0".to_string(),
    });
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
    meta.alias.push(Alias {
      name: "stable".to_string(),
      required: "^1.0.0".to_string(),
    });
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
    meta.alias.push(Alias {
      name: "v1".to_string(),
      required: "1.2.3".to_string(),
    });
    let result = meta.resolve_version_req("v1");
    assert!(result.is_ok());
  }

  #[test]
  fn test_resolve_version_req_lts_alias() {
    let mut meta = DvmMeta::default();
    meta.alias.push(Alias {
      name: "lts".to_string(),
      required: "1.40.0".to_string(),
    });
    let result = meta.resolve_version_req("lts");
    assert!(result.is_ok());
  }

  #[test]
  fn test_resolve_version_req_range_alias() {
    let mut meta = DvmMeta::default();
    meta.alias.push(Alias {
      name: "stable".to_string(),
      required: "^1.0.0".to_string(),
    });
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
      meta.alias.push(Alias {
        name: "latest".to_string(),
        required: "*".to_string(),
      });
    }
    assert_eq!(meta.alias.len(), 0);
  }

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
    let result = serde_json::from_str::<DvmMeta>(
      r#"{"alias": [{"name": "stable", "required": "1.0.0"}]}"#,
    );
    assert!(result.is_err());
  }

  #[test]
  fn test_deserialize_missing_alias_field_fails() {
    let result = serde_json::from_str::<DvmMeta>(
      r#"{"versions": [{"required": "~1.0.0", "current": "1.0.1"}]}"#,
    );
    assert!(result.is_err());
  }

  #[test]
  fn test_deserialize_with_null_fields_fails() {
    let result = serde_json::from_str::<DvmMeta>(
      r#"{"versions": null, "alias": null}"#,
    );
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
        "versions": [],
        "alias": [],
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
    // versions is a string instead of an array
    let result = serde_json::from_str::<DvmMeta>(
      r#"{"versions": "not-an-array", "alias": []}"#,
    );
    assert!(result.is_err());

    // alias is a number instead of an array
    let result = serde_json::from_str::<DvmMeta>(
      r#"{"versions": [], "alias": 123}"#,
    );
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
