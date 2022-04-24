// Copyright 2018-2020 the Deno authors. All rights reserved. MIT license.
// Copyright 2020-2021 justjavac. All rights reserved. MIT license.
use super::use_version;
use crate::utils::{deno_bin_path, dvm_root, is_china_mainland};
use anyhow::Result;
use semver::Version;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string::String;

#[cfg(windows)]
const ARCHIVE_NAME: &str = "deno-x86_64-pc-windows-msvc.zip";
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const ARCHIVE_NAME: &str = "deno-aarch64-apple-darwin.zip";
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const ARCHIVE_NAME: &str = "deno-x86_64-apple-darwin.zip";
#[cfg(target_os = "linux")]
const ARCHIVE_NAME: &str = "deno-x86_64-unknown-linux-gnu.zip";

pub fn exec(no_use: bool, version: Option<String>) -> Result<()> {
  let install_version = match version {
    Some(passed_version) => match Version::parse(&passed_version) {
      Ok(ver) => ver,
      Err(_) => {
        eprintln!("Invalid semver");
        std::process::exit(1)
      }
    },
    None => get_latest_version()?,
  };

  let exe_path = deno_bin_path(&install_version);

  if exe_path.exists() {
    println!("version v{} is already installed", install_version);
  } else {
    let archive_data = download_package(&compose_url_to_exec(&install_version), &install_version)?;
    unpack(archive_data, &install_version)?;
  }

  if !no_use {
    use_version::use_this_bin_path(&exe_path, &install_version)?;
  }

  Ok(())
}

fn get_latest_version() -> Result<Version> {
  println!("Checking for latest version");
  let response = if is_china_mainland() {
    tinyget::get("https://dl.deno.js.cn/release-latest.txt").send()?
  } else {
    tinyget::get("https://dl.deno.land/release-latest.txt").send()?
  };

  let body = response.as_str()?;
  let v = body.trim().replace('v', "");
  println!("The latest version is v{}", &v);
  Ok(Version::parse(&v).unwrap())
}

fn download_package(url: &str, version: &Version) -> Result<Vec<u8>> {
  println!("downloading {}", &url);

  let response = match tinyget::get(url).send() {
    Ok(response) => response,
    Err(error) => {
      println!("Network error {}", &error);
      std::process::exit(1)
    }
  };

  if response.status_code == 404 {
    println!("Version has not been found, aborting");
    std::process::exit(1)
  }

  if response.status_code >= 400 && response.status_code <= 599 {
    println!("Download '{}' failed: {}", &url, response.status_code);
    std::process::exit(1)
  }

  println!("Version has been found");
  println!("Deno v{} has been downloaded", &version);

  Ok(response.into_bytes())
}

fn compose_url_to_exec(version: &Version) -> String {
  if is_china_mainland() {
    format!("https://dl.deno.js.cn/release/v{}/{}", version, ARCHIVE_NAME)
  } else {
    format!("https://dl.deno.land/release/v{}/{}", version, ARCHIVE_NAME)
  }
}

fn unpack(archive_data: Vec<u8>, version: &Version) -> Result<PathBuf> {
  let dvm_dir = dvm_root().join(format!("{}", version));
  fs::create_dir_all(&dvm_dir)?;
  let exe_path = deno_bin_path(version);

  let archive_ext = Path::new(ARCHIVE_NAME)
    .extension()
    .and_then(|ext| ext.to_str())
    .unwrap();
  let unpack_status = match archive_ext {
    "zip" if cfg!(windows) => {
      let archive_path = dvm_dir.join("deno.zip");
      fs::write(&archive_path, &archive_data)?;
      Command::new("powershell.exe")
        .arg("-NoLogo")
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(
          "& {
            param($Path, $DestinationPath)
            trap { $host.ui.WriteErrorLine($_.Exception); exit 1 }
            Add-Type -AssemblyName System.IO.Compression.FileSystem
            [System.IO.Compression.ZipFile]::ExtractToDirectory(
              $Path,
              $DestinationPath
            );
          }",
        )
        .arg("-Path")
        .arg(format!("'{}'", &archive_path.to_str().unwrap()))
        .arg("-DestinationPath")
        .arg(format!("'{}'", &dvm_dir.to_str().unwrap()))
        .spawn()?
        .wait()?
    }
    "zip" => {
      let archive_path = dvm_dir.join("deno.zip");
      fs::write(&archive_path, &archive_data)?;
      Command::new("unzip")
        .current_dir(&dvm_dir)
        .arg(archive_path)
        .spawn()?
        .wait()?
    }
    ext => panic!("Unsupported archive type: '{}'", ext),
  };
  assert!(unpack_status.success());
  assert!(exe_path.exists());
  Ok(exe_path)
}

#[test]
fn test_compose_url_to_exec() {
  let v = Version::parse("1.7.0").unwrap();
  let url = compose_url_to_exec(&v);
  #[cfg(windows)]
  assert!(
    url.as_str() == "https://dl.deno.land/release/v1.7.0/deno-x86_64-pc-windows-msvc.zip"
      || url.as_str() == "https://dl.deno.js.cn/release/v1.7.0/deno-x86_64-pc-windows-msvc.zip"
  );

  #[cfg(target_os = "macos")]
  assert!(
    url.as_str() == "https://dl.deno.land/release/v1.7.0/deno-x86_64-apple-darwin.zip"
      || url.as_str() == "https://dl.deno.js.cn/release/v1.7.0/deno-x86_64-apple-darwin.zip"
      || url.as_str() == "https://dl.deno.land/release/v1.7.0/deno-aarch64-apple-darwin.zip"
      || url.as_str() == "https://dl.deno.js.cn/release/v1.7.0/deno-aarch64-apple-darwin.zip"
  );

  #[cfg(target_os = "linux")]
  assert!(
    url.as_str() == "https://dl.deno.land/release/v1.7.0/deno-x86_64-unknown-linux-gnu.zip"
      || url.as_str() == "https://dl.deno.js.cn/release/v1.7.0/deno-x86_64-unknown-linux-gnu.zip"
  );
}
