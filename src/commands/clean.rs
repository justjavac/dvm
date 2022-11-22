use crate::configrc::rc_clean;
use crate::{dvm_root, DvmMeta};
use anyhow::Result;

pub fn exec(meta: &mut DvmMeta) -> Result<()> {
  let home = dvm_root();

  let cache_folder = home.join("versions");
  if !cache_folder.exists() {
    std::process::exit(0);
  }

  let requires = meta
    .versions
    .iter()
    .filter_map(|v| {
      if v.is_valid_mapping() {
        None
      } else {
        Some(v.required.clone())
      }
    })
    .collect::<Vec<_>>();

  for required in requires {
    meta.delete_version_mapping(required.clone());
  }

  meta.clean_files();

  // clean user-wide rc file
  rc_clean(true).expect("clean local rc file failed");
  rc_clean(false).expect("clean user-wide rc file failed");

  println!("Cleaned successfully");
  Ok(())
}
