use anyhow::Result;

use crate::version::{get_local_versions, get_remote_versions};

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
  versions.sort();
  versions.reverse();
  println!("{}", versions.join("\n"));
}
