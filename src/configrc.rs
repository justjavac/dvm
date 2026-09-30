use crate::consts::{
  DVM_CONFIGRC_FILENAME, DVM_CONFIGRC_KEY_DENO_VERSION, DVM_CONFIGRC_KEY_REGISTRY_BINARY,
  DVM_CONFIGRC_KEY_REGISTRY_VERSION,
};
use crate::consts::{REGISTRY_LIST_OFFICIAL, REGISTRY_OFFICIAL};
use std::fs;
use std::io;

/// check global rc file exists
pub fn rc_exists() -> bool {
  let dir = dirs::home_dir()
    .map(|it| it.join(DVM_CONFIGRC_FILENAME))
    .unwrap_or_default();
  fs::metadata(dir).is_ok()
}

/// init user-wide rc file
pub fn rc_init() -> io::Result<()> {
  rc_update(false, DVM_CONFIGRC_KEY_REGISTRY_BINARY, REGISTRY_OFFICIAL)?;
  rc_update(false, DVM_CONFIGRC_KEY_REGISTRY_VERSION, REGISTRY_LIST_OFFICIAL)
}

/// fix missing rc properties
pub fn rc_fix() -> io::Result<()> {
  if !rc_exists() {
    rc_init()?;
  } else {
    if !rc_has(DVM_CONFIGRC_KEY_REGISTRY_BINARY) {
      rc_update(false, DVM_CONFIGRC_KEY_REGISTRY_BINARY, REGISTRY_OFFICIAL)?;
    }
    if !rc_has(DVM_CONFIGRC_KEY_REGISTRY_VERSION) {
      rc_update(false, DVM_CONFIGRC_KEY_REGISTRY_VERSION, REGISTRY_LIST_OFFICIAL)?;
    }
    if !rc_has(DVM_CONFIGRC_KEY_DENO_VERSION) {
      rc_update(false, DVM_CONFIGRC_KEY_DENO_VERSION, "latest")?;
    }
  }

  Ok(())
}

/// check if key exists in rc file
pub fn rc_has(key: &str) -> bool {
  let Ok(content) = rc_content_cascade() else {
    return false;
  };

  rc_parse(content.as_str()).iter().any(|(k, _)| *k == key)
}

/// Source of a dvmrc configuration value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RcSource {
  /// Value came from the local (current-directory) .dvmrc file.
  Local,
  /// Value came from the user-wide (home-directory) .dvmrc file.
  Global,
}

/// get value by key from configrc, along with which file it came from.
/// first try to get from current folder
/// if not found, try to get from home folder
/// if not found, return Err
pub fn rc_get_with_source(key: &str) -> io::Result<(String, RcSource)> {
  // Try local .dvmrc first
  if let Ok(content) = rc_read(true) {
    let config = rc_parse(&content);
    if let Some((_, v)) = config.iter().find(|(k, _)| *k == key) {
      return Ok((v.to_string(), RcSource::Local));
    }
  }
  // Fall back to global .dvmrc
  if let Ok(content) = rc_read(false) {
    let config = rc_parse(&content);
    if let Some((_, v)) = config.iter().find(|(k, _)| *k == key) {
      return Ok((v.to_string(), RcSource::Global));
    }
  }
  Err(io::Error::new(io::ErrorKind::NotFound, "key not found"))
}

/// get value by key from configrc
/// first try to get from current folder
/// if not found, try to get from home folder
/// if not found, return Err
pub fn rc_get(key: &str) -> io::Result<String> {
  if !rc_exists() {
    rc_init()?;
  }

  let content = rc_content_cascade()?;
  let config = rc_parse(content.as_str());

  config
    .iter()
    .find_map(|(k, v)| if k == &key { Some(v.to_string()) } else { None })
    .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "key not found"))
}

/// get value by key from configuration with a possible fix
/// first try to get from current folder
/// if not found, try to get from home folder
/// if not found, try to the fix the missing properties.
/// and then try to get this key's value again without the fix
pub fn rc_get_with_fix(key: &str) -> io::Result<String> {
  // always return the error which is from `rc_get` fn
  rc_get(key).or_else(|err| rc_fix().and_then(|_| rc_get(key)).map_err(|_| err))
}

/// update the config file key with the new value
/// create the file if it doesn't exist
/// create key value pair if it doesn't exist
pub fn rc_update(is_local: bool, key: &str, value: &str) -> io::Result<()> {
  let (config_path, content) = rc_content(is_local)?;

  let mut config: Vec<(&str, &str)> = content
    .as_ref()
    .map(|c| rc_parse(c.as_str()))
    .unwrap_or_default();

  let idx = config.iter().position(|(k, _)| k == &key);
  if let Some(idx) = idx {
    config[idx].1 = value;
  } else {
    config.push((key, value));
  }

  let config = config
    .iter()
    .map(|(k, v)| format!("{}={}", k, v))
    .collect::<Vec<_>>()
    .join("\n");
  fs::write(config_path, config)
}

fn rc_parse(content: &str) -> Vec<(&str, &str)> {
  content
    .lines()
    // throw away non key value pair
    .filter(|it| it.contains('='))
    .map(|line| {
      let mut parts = line.splitn(2, '=');
      let k = parts.next().unwrap().trim();
      let v = parts.next().unwrap_or("").trim();
      (k, v)
    })
    .collect::<Vec<_>>()
}

/// Path of the local (current directory) or user-wide rc file.
fn rc_path(is_local: bool) -> io::Result<std::path::PathBuf> {
  if is_local {
    return Ok(std::path::PathBuf::from(DVM_CONFIGRC_FILENAME));
  }

  dirs::home_dir()
    .map(|home| home.join(DVM_CONFIGRC_FILENAME))
    .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "could not determine the home directory"))
}

fn rc_content(is_local: bool) -> io::Result<(std::path::PathBuf, io::Result<String>)> {
  let config_path = rc_path(is_local)?;
  let content = fs::read_to_string(&config_path);

  Ok((config_path, content))
}

/// Read the local or user-wide rc file.
fn rc_read(is_local: bool) -> io::Result<String> {
  fs::read_to_string(rc_path(is_local)?)
}

/// Read merged config: global (user-wide) values form the base, and local
/// (current directory) values override on a per-key basis.  This way a project
/// `.dvmrc` that only sets `deno_version` does not erase the user's registry
/// preferences.
fn rc_content_cascade() -> io::Result<String> {
  let global = rc_read(false).unwrap_or_default();
  let local = rc_read(true).unwrap_or_default();

  if local.is_empty() {
    return if global.is_empty() {
      Err(io::Error::new(io::ErrorKind::NotFound, "no rc file found"))
    } else {
      Ok(global)
    };
  }

  // Start from global pairs, then overlay local pairs.
  let global_pairs = rc_parse(&global);
  let mut merged: std::collections::HashMap<String, String> = global_pairs
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

  for (k, v) in rc_parse(&local) {
    merged.insert(k.to_string(), v.to_string());
  }

  // Preserve original global ordering, append any new local keys at the end.
  let mut result: Vec<String> = Vec::new();
  for (k, _) in rc_parse(&global) {
    if let Some(v) = merged.get(k) {
      result.push(format!("{}={}", k, v));
    }
  }
  for (k, v) in rc_parse(&local) {
    if !result.iter().any(|line| line.starts_with(&format!("{}=", k))) {
      result.push(format!("{}={}", k, v));
    }
  }

  Ok(result.join("\n"))
}

/// remove all key value pair that ain't supported by dvm from config file
pub fn rc_clean(is_local: bool) -> io::Result<()> {
  if !rc_exists() {
    rc_init()?;
  }

  let (config_path, content) = rc_content(is_local)?;
  let content = if let Ok(content) = content {
    content
  } else {
    // if file not found, just return Ok, 'cause it's not needed to be cleaned
    return Ok(());
  };

  let config = rc_parse(content.as_str());
  let config = config
    .iter()
    .filter(|(k, _)| {
      k == &DVM_CONFIGRC_KEY_DENO_VERSION
        || k == &DVM_CONFIGRC_KEY_REGISTRY_BINARY
        || k == &DVM_CONFIGRC_KEY_REGISTRY_VERSION
    })
    .collect::<Vec<_>>();

  let config = config
    .iter()
    .map(|(k, v)| format!("{}={}", k, v))
    .collect::<Vec<_>>()
    .join("\n");
  fs::write(config_path, config)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rc_parse_trims_keys_and_values() {
    let config = rc_parse(" deno_version = 1.2.3 \nregistry_binary = https://example.com/ \n");

    assert_eq!(
      config,
      vec![("deno_version", "1.2.3"), ("registry_binary", "https://example.com/")]
    );
  }

  #[test]
  fn rc_merge_local_overrides_global_per_key() {
    // Simulate a global config with registry settings and a local config
    // that only overrides deno_version.  The merged result should contain
    // all three keys — this is the bug fix: previously a local .dvmrc
    // would completely replace global config, losing registry settings.
    let global = "registry_binary=https://example.com/\nregistry_version=https://example.com/versions.json";
    let local = "deno_version=1.0.0";

    let global_pairs = rc_parse(global);
    let mut merged: std::collections::HashMap<String, String> = global_pairs
      .into_iter()
      .map(|(k, v)| (k.to_string(), v.to_string()))
      .collect();
    for (k, v) in rc_parse(local) {
      merged.insert(k.to_string(), v.to_string());
    }

    assert_eq!(merged.len(), 3);
    assert_eq!(merged.get("registry_binary").unwrap(), "https://example.com/");
    assert_eq!(
      merged.get("registry_version").unwrap(),
      "https://example.com/versions.json"
    );
    assert_eq!(merged.get("deno_version").unwrap(), "1.0.0");
  }

  #[test]
  fn rc_merge_local_overrides_same_key() {
    let global = "deno_version=2.0.0\nregistry_binary=https://global.example.com/";
    let local = "deno_version=1.0.0";

    let global_pairs = rc_parse(global);
    let mut merged: std::collections::HashMap<String, String> = global_pairs
      .into_iter()
      .map(|(k, v)| (k.to_string(), v.to_string()))
      .collect();
    for (k, v) in rc_parse(local) {
      merged.insert(k.to_string(), v.to_string());
    }

    // local deno_version overrides global, but registry_binary stays global
    assert_eq!(merged.get("deno_version").unwrap(), "1.0.0");
    assert_eq!(merged.get("registry_binary").unwrap(), "https://global.example.com/");
  }
}
