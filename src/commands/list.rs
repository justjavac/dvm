use anyhow::Result;

use crate::version::{
  get_current_version, get_local_versions, get_remote_versions,
};

pub fn exec() -> Result<()> {
  let versions = get_local_versions();

  print_versions(versions);
  Ok(())
}

pub fn exec_remote() -> Result<()> {
  let versions = get_remote_versions().unwrap();

  print_versions(versions);
  Ok(())
}

fn print_versions(mut versions: Vec<String>) {
  let current_version = match get_current_version() {
    Some(v) => v,
    _ => String::from(""),
  };
  versions.sort();
  versions.reverse();

  for v in &versions {
    if v.to_string() == current_version {
      // display current used version with bright green
      println!("\x1b[0;92m*{}\x1b[0m", v);
    } else {
      println!(" {}", v)
    }
  }
}
