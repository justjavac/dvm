use std::process;

use crate::consts::REGISTRY_CN;
use crate::consts::REGISTRY_NAME_CN;
use crate::consts::REGISTRY_NAME_OFFICIAL;
use crate::consts::REGISTRY_OFFICIAL;
use crate::DvmMeta;

use anyhow::Result;

pub fn exec(meta: &mut DvmMeta, registry: Option<String>) -> Result<()> {
  let registry = registry.unwrap_or_else(|| REGISTRY_NAME_OFFICIAL.to_string());

  if registry == *REGISTRY_NAME_OFFICIAL {
    meta.registry = REGISTRY_OFFICIAL.to_string();
    println!("Registry now set to the official registry \"{}\"", REGISTRY_OFFICIAL);
  } else if registry == *REGISTRY_NAME_CN {
    meta.registry = REGISTRY_CN.to_string();
    println!(
      "Registry now set to the CN mirror (that provided by @justjavac) \"{}\"",
      REGISTRY_CN
    )
  } else if registry.starts_with("http://") || registry.starts_with("https://") {
    meta.registry = registry;
  } else {
    eprintln!(
      "The {} is not valid URL, please starts with `http` or `https`",
      registry
    );
    eprintln!("Registry will not be changed");
    process::exit(1)
  }

  meta.save();
  Ok(())
}
