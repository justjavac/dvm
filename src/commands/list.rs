// Copyright 2020 justjavac. All rights reserved. MIT license.
use anyhow::Result;

use crate::version::get_local_versions;

pub fn exec() -> Result<()> {
  let mut versions = get_local_versions();
  versions.sort();
  versions.reverse();
  println!("{}", versions.join("\n"));
  Ok(())
}
