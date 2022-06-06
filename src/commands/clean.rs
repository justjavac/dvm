use crate::consts::DVM_CACHE_INVALID_TIMEOUT;
use crate::utils::now;
use crate::{dvm_root, DvmMeta};
use anyhow::Result;
use colored::Colorize;

pub fn exec(meta: &mut DvmMeta) -> Result<()> {
  let home = dvm_root();

  let cache_folder = home.join("versions");
  if !cache_folder.exists() {
    std::process::exit(0);
  }

  if let Ok(dir) = cache_folder.read_dir() {
    for entry in dir.flatten() {
      let path = entry.path();
      if path.is_dir() {
        let name = path.file_name().unwrap().to_str().unwrap();

        // it's been pointed by dvm versions
        if meta.versions.iter().any(|it| it.current == name) {
          continue;
        }

        // it's not been outdated
        let stub = path.join(".dvmstub");
        if stub.exists() && stub.is_file() {
          let content = std::fs::read_to_string(stub).expect("read stub file failed");
          let content: u128 = content.parse().expect("parse stub file failed");
          if content > now() - DVM_CACHE_INVALID_TIMEOUT {
            continue;
          }
        }

        println!("Cleaning version {}", name.bright_black());
        std::fs::remove_dir_all(path).unwrap();
      }
    }
  }

  println!("Cleaned successfully");
  Ok(())
}
