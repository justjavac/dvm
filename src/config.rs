#![allow(dead_code)]
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use semver::{Version, VersionReq};

#[derive(Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct VersionMapping {
    required: String,
    current: String,
}
impl VersionMapping {
    pub(crate) fn to_version_req(&self) -> VersionReq {
        VersionReq::from_str(&self.required).expect("VersionMapping::required is not a valid VersionReq")
    }

    pub(crate) fn try_to_version_req(&self) -> anyhow::Result<VersionReq> {
        VersionReq::from_str(&self.required).map_err(|err| anyhow::anyhow!(err))
    }

    pub(crate) fn is_valid_mapping(&self) -> bool {
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

#[derive(Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct Alias {
    name: String,
    required: String,
}
impl Alias {
    pub(crate) fn to_version_req(&self) -> VersionReq {
        VersionReq::from_str(&self.required).expect("Alias::required is not a valid VersionReq")
    }

    pub(crate) fn try_to_version_req(&self) -> anyhow::Result<VersionReq> {
        VersionReq::from_str(&self.required).map_err(|err| anyhow::anyhow!(err))
    }
}

#[derive(Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct DvmConfig {
    versions: Vec<VersionMapping>,
    alias: Vec<Alias>,
}

impl Default for DvmConfig {
    fn default() -> Self {
        Self {
            versions: vec![],
            alias: vec![],
        }
    }
}


#[cfg(test)]
mod tests{
    use serde_json::json;
    use super::*;

    #[test]
    fn test_default_config() {
        let result = serde_json::to_string(&DvmConfig::default());
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            "{\"versions\":[],\"alias\":[]}"
        );
    }

    #[test]
    fn test_versions_config() {
        let mut conf = DvmConfig::default();
        conf.versions.push(VersionMapping{
            required: "~1.0.0".to_string(),
            current: "1.0.1".to_string()
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
        let mut conf = DvmConfig::default();
        conf.alias.push(Alias{
            name: "stable".to_string(),
            required: "1.0.0".to_string()
        });
        conf.alias.push(Alias {
            name: "two-point-o".to_string(),
            required: "2.0.0".to_string()
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

        let parsed = DvmConfig::deserialize(raw);
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