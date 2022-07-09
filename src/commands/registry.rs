use std::process;

use crate::consts::REGISTRY_CN;
use crate::consts::REGISTRY_OFFICIAL;
use crate::DvmMeta;

use anyhow::Result;

pub fn exec(meta: &mut DvmMeta, registry: Option<String>) -> Result<()> {
  let registry = registry.unwrap_or("official".to_string());

  if registry == "official".to_string() {
    meta.registry = REGISTRY_OFFICIAL.to_string();
  } else if registry == "cn".to_string() {
    meta.registry = REGISTRY_CN.to_string();
  } else if registry.starts_with("http://") || registry.starts_with("https://") {
    meta.registry = registry;
  } else {
    eprintln!("{} is not valid URL, please starts with `http` or `https`", registry);
    eprintln!("registry will not be changed");
    process::exit(1)
  }

  meta.save();
  Ok(())
}
