use crate::utils::{get_dvm_root, get_exe_path};
use std::fs;
use std::process::{exit, Command};

use anyhow::Result;
use semver_parser::version::parse as semver_parse;

pub fn exec(version: Option<String>) -> Result<()> {
  let target_version = match version {
    Some(target_version) => match semver_parse(&target_version) {
      Ok(ver) => ver,
      Err(_) => {
        eprintln!("Invalid semver");
        std::process::exit(1)
      }
    },
    None => unimplemented!(),
  };

  let target_exe_path = get_exe_path(&target_version);

  if !target_exe_path.exists() {
    eprintln!("deno v{} is not installed.", target_version);
    std::process::exit(1)
  }

  let deno_v_output = Command::new("deno")
    .arg("-V")
    .output()
    .expect("deno has not been installed yet.");
  let output_str = String::from_utf8_lossy(&deno_v_output.stdout);

  if output_str.trim() == format!("deno {}", target_version) {
    println!("Failed: deno v{} is in use.", target_version);
    exit(1);
  }

  let dvm_dir = get_dvm_root().join(format!("{}", target_version));

  fs::remove_dir_all(&dvm_dir).unwrap();
  println!("deno v{} removed.", target_version);

  Ok(())
}
