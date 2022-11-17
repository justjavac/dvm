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

/// get value by key from configrc
/// first try to get from current folder
/// if not found, try to get from home folder
/// if not found, return Err
pub fn rc_get(key: &str) -> io::Result<String> {
  if !rc_exists() {
    rc_init()?;
  }

  let (_, content) = rc_content(true).or_else(|_| rc_content(false))?;
  let config = rc_parse(content.as_str());

  config
    .iter()
    .find_map(|(k, v)| if k == &key { Some(v.to_string()) } else { None })
    .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "key not found"))
}

/// update the config file key with the new value
/// create the file if it doesn't exist
/// create key value pair if it doesn't exist
#[allow(dead_code)]
pub fn rc_update(is_local: bool, key: &str, value: &str) -> io::Result<()> {
  let (config_path, content) = rc_content(is_local).unwrap_or_default();
  let mut config = rc_parse(content.as_str());

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

/// remove key value pair from config file
#[allow(dead_code)]
pub fn rc_remove(is_local: bool, key: &str) -> io::Result<()> {
  let (config_path, content) = rc_content(is_local).unwrap_or_default();
  let config = rc_parse(content.as_str());
  let config = config.iter().filter(|(k, _)| k != &key).collect::<Vec<_>>();

  let config = config
    .iter()
    .map(|(k, v)| format!("{}={}", k, v))
    .collect::<Vec<_>>()
    .join("\n");
  fs::write(config_path, config)
}

fn rc_parse(content: &str) -> Vec<(&str, &str)> {
  let config = content
    .lines()
    .map(|line| {
      let mut parts = line.splitn(2, '=');
      let k = parts.next().unwrap();
      let v = parts.next().unwrap_or("");
      (k, v)
    })
    .collect::<Vec<_>>();
  config
}

fn rc_content(is_local: bool) -> io::Result<(std::path::PathBuf, String)> {
  let config_path = if is_local {
    std::path::PathBuf::from(DVM_CONFIGRC_FILENAME)
  } else {
    dirs::home_dir()
      .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "helllllll"))?
      .join(DVM_CONFIGRC_FILENAME)
  };

  Ok((config_path.clone(), fs::read_to_string(config_path).unwrap_or_default()))
}

/// remove all key value pair that ain't supported by dvm from config file
#[allow(dead_code)]
pub fn rc_clean(is_local: bool) -> io::Result<()> {
  let (config_path, content) = rc_content(is_local).unwrap_or_default();
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

/// clear and delete the rc file
/// if is_local is true, delete the local rc file
/// if is_local is false, delete the global(user-wide) rc file
#[allow(dead_code)]
pub fn rc_clear(is_local: bool) -> io::Result<()> {
  if is_local {
    std::fs::remove_file(DVM_CONFIGRC_FILENAME)
  } else {
    let home_dir = dirs::home_dir().ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
    let rc_file = home_dir.join(DVM_CONFIGRC_FILENAME);
    std::fs::remove_file(rc_file)
  }
}
