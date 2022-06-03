#![allow(dead_code)]
use crate::consts::{DVM_CACHE_PATH_PREFIX, REGISTRY_OFFICIAL};
use crate::utils::{deno_version_path, dvm_root};
use crate::version::VersionArg;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, read_to_string, write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub const DEFAULT_ALIAS: phf::Map<&'static str, &'static str> = phf::phf_map! {
  "latest" => "*"
};

pub trait ToVersionReq {
  fn to_version_req(&self) -> VersionReq;
  fn try_to_version_req(&self) -> anyhow::Result<VersionReq>;
}

#[derive(Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct VersionMapping {
  required: String,
  current: String,
}
impl VersionMapping {
  pub fn is_valid_mapping(&self) -> bool {
    let current = Version::parse(&self.current);
    if current.is_err() {
      return false;
    }
    let current = current.unwrap();

    let req = self.try_to_version_req();
    if req.is_err() {
      return false;
    }
    let req = req.unwrap();

    req.matches(&current)
  }
}

impl ToVersionReq for VersionMapping {
  fn to_version_req(&self) -> VersionReq {
    VersionReq::from_str(&self.required).expect("VersionMapping::required is not a valid VersionReq")
  }

  fn try_to_version_req(&self) -> anyhow::Result<VersionReq> {
    VersionReq::from_str(&self.required).map_err(|err| anyhow::anyhow!(err))
  }
}

#[derive(Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct Alias {
  pub name: String,
  pub required: String,
}

impl ToVersionReq for Alias {
  fn to_version_req(&self) -> VersionReq {
    VersionReq::from_str(&self.required).expect("Alias::required is not a valid VersionReq")
  }

  fn try_to_version_req(&self) -> anyhow::Result<VersionReq> {
    VersionReq::from_str(&self.required).map_err(|err| anyhow::anyhow!(err))
  }
}

#[derive(Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct DvmMeta {
  #[serde(default = "default_registry")]
  pub registry: String,
  pub versions: Vec<VersionMapping>,
  pub alias: Vec<Alias>,
}

pub fn default_registry() -> String {
  REGISTRY_OFFICIAL.to_string()
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
        if let Ok(mut config) = config {
          let mut i = 0;
          while i < config.versions.len() {
            if !deno_version_path(&Version::parse(&config.versions[i].current).unwrap()).exists() {
              config.versions.remove(i);
            } else {
              i += 1;
            }
          }
          return config;
        }
      }
    }

    let config = DvmMeta::default();
    config.save();
    config
  }

  pub fn clean_files(&self) {
    let bin_path = dvm_root().join(Path::new(DVM_CACHE_PATH_PREFIX));
    for entry in bin_path.read_dir().expect("read_dir call failed").flatten() {
      if entry.metadata().unwrap().is_dir()
        && !self
          .versions
          .iter()
          .map(|it| it.current.clone())
          .any(|x| x == *entry.file_name().to_str().unwrap())
      {
        std::fs::remove_dir_all(entry.path()).unwrap();
      }
    }
  }

  ///
  /// set a version mapping
  ///   `required` is either a semver range or a alias to a semver rage
  ///   `current` is the current directory that the deno located in
  pub fn set_version_mapping(&mut self, required: String, current: String) {
    println!("{}, {}", &required, current);
    let result = self.versions.iter().position(|it| it.required == required);
    if let Some(index) = result {
      self.versions[index] = VersionMapping { required, current };
    } else {
      self.versions.push(VersionMapping { required, current });
    }
    self.save();
  }

  ///
  /// get fold name of a given mapping,
  /// None if there haven't a deno version met the required semver range or alias that
  /// are installed already
  pub fn get_version_mapping(&self, required: &str) -> Option<String> {
    self
      .versions
      .iter()
      .position(|it| it.required == required)
      .map(|index| self.versions[index].current.clone())
  }

  ///
  /// delete a version mapping
  /// this will also delete actual files.
  pub fn delete_version_mapping(&mut self, required: String) {
    let result = self.versions.iter().position(|it| it.required == required);
    if let Some(index) = result {
      self.versions.remove(index);
    }

    self.save();
  }

  /// set a alias
  ///   name is alias name
  ///   required is a semver range
  pub fn set_alias(&mut self, name: String, required: String) {
    if DEFAULT_ALIAS.contains_key(name.as_str()) {
      return;
    }
    let result = self.alias.iter().position(|it| it.name == name);
    if let Some(index) = result {
      self.alias[index] = Alias { name, required };
    } else {
      self.alias.push(Alias { name, required });
    }

    self.save();
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
        .position(|it| it.name == name)
        .map(|index| VersionArg::from_str(&self.alias[index].required).unwrap())
    }
  }

  /// delete a alias
  pub fn delete_alias(&mut self, name: String) {
    let result = self.alias.iter().position(|it| it.name == name);
    if let Some(index) = result {
      self.alias.remove(index);
    }

    self.save();
  }

  pub fn resolve_version_req(&self, required: &str) -> VersionArg {
    if self.has_alias(required) {
      self.get_alias(required).unwrap()
    } else {
      VersionArg::from_str(required).unwrap()
    }
  }

  /// reload from disk
  pub fn reload(&mut self) {
    let new = DvmMeta::new();
    self.versions = new.versions;
    self.alias = new.alias;
  }

  /// write to disk
  pub fn save(&self) {
    let file_path = DvmMeta::path();
    let dir_path = file_path.parent().unwrap();
    if !dir_path.exists() {
      create_dir_all(dir_path).unwrap();
    }
    write(file_path, serde_json::to_string_pretty(self).unwrap()).unwrap();
  }
}

impl<'a> Default for DvmMeta {
  fn default() -> Self {
    Self {
      registry: REGISTRY_OFFICIAL.to_string(),
      versions: vec![],
      alias: vec![],
    }
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
    assert_eq!(
      result.unwrap(),
      "{\"registry\":\"https://dl.deno.land/\",\"versions\":[],\"alias\":[]}"
    );
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
      "{\"registry\":\"https://dl.deno.land/\",\"versions\":[{\"required\":\"~1.0.0\",\"current\":\"1.0.1\"}],\"alias\":[]}"
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
            "{\"registry\":\"https://dl.deno.land/\",\"versions\":[],\"alias\":[{\"name\":\"stable\",\"required\":\"1.0.0\"},{\"name\":\"two-point-o\",\"required\":\"2.0.0\"}]}"
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
}
