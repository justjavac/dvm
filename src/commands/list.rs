use anyhow::Result;
use semver_parser::version::parse;
use std::cmp::Ordering;

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

  versions.sort_by(|a, b| sort_semver_version(b, a));

  for v in &versions {
    if v.to_string() == current_version {
      // display current used version with bright green
      println!("\x1b[0;92m*{}\x1b[0m", v);
    } else {
      println!(" {}", v)
    }
  }
}

fn sort_semver_version(s1: &String, s2: &String) -> Ordering {
  let v1 = parse(s1).unwrap();
  let v2 = parse(s2).unwrap();

  if v1.major > v2.major {
    return Ordering::Greater;
  }

  if v1.major < v2.major {
    return Ordering::Less;
  }
  if v1.minor > v2.minor {
    return Ordering::Greater;
  }

  if v1.minor < v2.minor {
    return Ordering::Less;
  }

  if v1.patch > v2.patch {
    return Ordering::Greater;
  }

  if v1.patch < v2.patch {
    return Ordering::Less;
  }

  Ordering::Equal
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn sort_version() {
    let v1 = String::from("0.2.10-beta");
    let v2 = String::from("0.2.9");
    let v3 = String::from("0.2.10-alpha1");
    let v4 = String::from("1.10.0");
    let v5 = String::from("1.9.2");
    let v6 = String::from("9.9.0");
    let v7 = String::from("10.9.0");

    assert_eq!(sort_semver_version(&v1, &v2), Ordering::Greater);
    assert_eq!(sort_semver_version(&v1, &v3), Ordering::Equal);
    assert_eq!(sort_semver_version(&v4, &v3), Ordering::Greater);
    assert_eq!(sort_semver_version(&v5, &v4), Ordering::Less);
    assert_eq!(sort_semver_version(&v7, &v6), Ordering::Greater);
  }
}
