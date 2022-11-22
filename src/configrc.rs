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

  content
    .lines()
    .filter(|it| it.contains('='))
    .map(|it| {
      let mut it = it.split('=');
      (it.next().unwrap().trim(), it.next().unwrap().trim())
    })
    .any(|(k, _)| k == key)
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

/// update the config file key with the new value
/// create the file if it doesn't exist
/// create key value pair if it doesn't exist
pub fn rc_update(is_local: bool, key: &str, value: &str) -> io::Result<()> {
  let (config_path, content) = rc_content(is_local);

  let _content;
  let mut config = if let Ok(c) = content {
    _content = c;
    rc_parse(_content.as_str())
  } else {
    Vec::new()
  };

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
  let (config_path, content) = rc_content(is_local);
  let Ok(content) = content else {
    // no need to remove
    return Ok(());
  };
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
    // throw away non key value pair
    .filter(|it| it.contains('='))
    .map(|line| {
      let mut parts = line.splitn(2, '=');
      let k = parts.next().unwrap();
      let v = parts.next().unwrap_or("");
      (k, v)
    })
    .collect::<Vec<_>>();
  config
}

fn rc_content(is_local: bool) -> (std::path::PathBuf, io::Result<String>) {
  let config_path = if is_local {
    std::path::PathBuf::from(DVM_CONFIGRC_FILENAME)
  } else {
    dirs::home_dir()
      .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
      .unwrap()
      .join(DVM_CONFIGRC_FILENAME)
  };

  (config_path.clone(), fs::read_to_string(config_path))
}

fn rc_content_cascade() -> io::Result<String> {
  rc_content(false).1.or_else(|_| rc_content(true).1)
}

/// remove all key value pair that ain't supported by dvm from config file
pub fn rc_clean(is_local: bool) -> io::Result<()> {
  if !rc_exists() {
    rc_init()?;
  }

  let (config_path, content) = rc_content(is_local);
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

/// clear and delete the rc file
/// if is_local is true, delete the local rc file
/// if is_local is false, delete the global(user-wide) rc file
#[allow(dead_code)]
pub fn rc_unlink(is_local: bool) -> io::Result<()> {
  if is_local {
    fs::remove_file(DVM_CONFIGRC_FILENAME)
  } else {
    let home_dir = dirs::home_dir().ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))?;
    let rc_file = home_dir.join(DVM_CONFIGRC_FILENAME);
    fs::remove_file(rc_file)
  }
}
