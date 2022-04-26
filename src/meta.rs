#![allow(dead_code)]
use crate::consts::REGISTRY_OFFICIAL;
use crate::utils::dvm_root;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::fs::{read_to_string, write};
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::str::FromStr;

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
pub(crate) struct Alias {
  name: String,
  required: String,
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
  pub registry: String,
  pub versions: Vec<VersionMapping>,
  pub alias: Vec<Alias>,
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
        let config = serde_json::from_str(content.as_str());
        if let Ok(config) = config {
          return config;
        }
      }
    }

    let config = DvmMeta::default();
    config.save();
    config
  }

  /// reload from disk
  pub fn reload(&mut self) {
    let new = DvmMeta::new();
    self.versions = new.versions;
    self.alias = new.alias;
  }

  /// write to disk
  pub fn save(&self) {
    write(DvmMeta::path(), serde_json::to_string_pretty(self).unwrap());
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
