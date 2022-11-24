// Copyright 2018-2020 the Deno authors. All rights reserved. MIT license.
// Copyright 2020-2022 justjavac. All rights reserved. MIT license.
use super::use_version;
use crate::configrc::rc_get;
use crate::consts::{
  DVM_CACHE_PATH_PREFIX, DVM_CANARY_PATH_PREFIX, DVM_CONFIGRC_KEY_REGISTRY_BINARY, DVM_VERSION_CANARY,
  DVM_VERSION_LATEST, REGISTRY_OFFICIAL,
};
use crate::meta::DvmMeta;
use crate::utils::{deno_canary_path, deno_version_path, dvm_root};
use crate::version::get_latest_canary;
use anyhow::Result;
use cfg_if::cfg_if;
use semver::Version;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string::String;

cfg_if! {
  if #[cfg(windows)] {
    const ARCHIVE_NAME: &str = "deno-x86_64-pc-windows-msvc.zip";
  } else if #[cfg(all(target_os = "macos", target_arch = "aarch64"))] {
    const ARCHIVE_NAME: &str = "deno-aarch64-apple-darwin.zip";
  } else if #[cfg(all(target_os = "macos", target_arch = "x86_64"))] {
    const ARCHIVE_NAME: &str = "deno-x86_64-apple-darwin.zip";
  } else if #[cfg(target_os = "linux")] {
    const ARCHIVE_NAME: &str = "deno-x86_64-unknown-linux-gnu.zip";
  }
}

pub fn exec(_: &DvmMeta, no_use: bool, version: Option<String>) -> Result<()> {
  let binary_registry_url = rc_get(DVM_CONFIGRC_KEY_REGISTRY_BINARY).unwrap_or_else(|_| REGISTRY_OFFICIAL.to_string());

  if let Some(version) = version.clone() {
    if version == *DVM_VERSION_CANARY {
      let canary_path = deno_canary_path();
      std::fs::create_dir_all(canary_path.parent().unwrap())?;
      let hash = get_latest_canary(&binary_registry_url).expect("Failed to get latest canary");
      let data = download_canary(&binary_registry_url, &hash)?;
      unpack_canary(data)?;

      if !no_use {
        use_version::use_canary_bin_path(false).unwrap();
      }

      return Ok(());
    }
  }

  let install_version = match version {
    Some(ref passed_version) => {
      Version::parse(passed_version).map_err(|_| anyhow::format_err!("Invalid semver {}", passed_version))?
    }
    None => get_latest_version(&binary_registry_url)?,
  };

  let exe_path = deno_version_path(&install_version);

  if exe_path.exists() {
    println!("Version v{} is already installed", install_version);
  } else {
    let archive_data = download_package(
      &compose_url_to_exec(&binary_registry_url, &install_version),
      &install_version,
    )?;
    unpack(archive_data, &install_version)?;
  }

  if !no_use {
    use_version::use_this_bin_path(
      &exe_path,
      &install_version,
      version.unwrap_or_else(|| DVM_VERSION_LATEST.to_string()),
      false,
    )?;
  }

  Ok(())
}

fn get_latest_version(registry: &str) -> Result<Version> {
  println!("Checking for latest version");

  let response = tinyget::get(format!("{}release-latest.txt", registry)).send()?;

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

fn compose_url_to_exec(registry: &str, version: &Version) -> String {
  format!("{}release/v{}/{}", registry, version, ARCHIVE_NAME)
}

fn unpack(archive_data: Vec<u8>, version: &Version) -> Result<PathBuf> {
  let version_dir = dvm_root().join(format!("{}/{}", DVM_CACHE_PATH_PREFIX, version));
  fs::create_dir_all(&version_dir)?;
  let exe_path = deno_version_path(version);

  unpack_impl(archive_data, version_dir, exe_path)
}

fn unpack_canary(archive_data: Vec<u8>) -> Result<PathBuf> {
  let canary_dir = dvm_root().join(DVM_CANARY_PATH_PREFIX);
  fs::create_dir_all(&canary_dir)?;
  let exe_path = deno_canary_path();

  if exe_path.exists() {
    fs::remove_file(exe_path.clone())?;
  }

  unpack_impl(archive_data, canary_dir, exe_path)
}

fn unpack_impl(archive_data: Vec<u8>, version_dir: PathBuf, path: PathBuf) -> Result<PathBuf> {
  let archive_ext = Path::new(ARCHIVE_NAME)
    .extension()
    .and_then(|ext| ext.to_str())
    .unwrap();
  let unpack_status = match archive_ext {
    "zip" if cfg!(windows) => {
      let archive_path = version_dir.join("deno.zip");
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
        .arg(format!("'{}'", &version_dir.to_str().unwrap()))
        .spawn()?
        .wait()?
    }
    "zip" => {
      let archive_path = version_dir.join("deno.zip");
      fs::write(&archive_path, &archive_data)?;
      Command::new("unzip")
        .current_dir(&version_dir)
        .arg(archive_path)
        .spawn()?
        .wait()?
    }
    ext => panic!("Unsupported archive type: '{}'", ext),
  };
  assert!(unpack_status.success());
  assert!(path.exists());
  Ok(version_dir)
}

fn download_canary(registry: &str, hash: &str) -> Result<Vec<u8>> {
  // TODO: remove this when deno canary support m1 chip,
  let archive_name = if ARCHIVE_NAME == "deno-aarch64-apple-darwin.zip" {
    "deno-x86_64-apple-darwin.zip"
  } else {
    ARCHIVE_NAME
  };

  let url = format!("{}canary/{}/{}", registry, hash, archive_name);

  let resp = tinyget::get(url).send()?;
  Ok(resp.into_bytes())
}

#[test]
fn test_compose_url_to_exec() {
  use crate::consts::REGISTRY_OFFICIAL;
  use asserts_rs::asserts_eq_one_of;

  let v = Version::parse("1.7.0").unwrap();
  let url = compose_url_to_exec(REGISTRY_OFFICIAL, &v);

  cfg_if! {
    if #[cfg(windows)] {
      asserts_eq_one_of!(
        url.as_str(),
        "https://dl.deno.land/release/v1.7.0/deno-x86_64-pc-windows-msvc.zip",
        "https://dl.deno.js.cn/release/v1.7.0/deno-x86_64-pc-windows-msvc.zip"
      );
    } else if #[cfg(target_os = "macos")] {
      asserts_eq_one_of!(
        url.as_str(),
        "https://dl.deno.land/release/v1.7.0/deno-x86_64-apple-darwin.zip",
        "https://dl.deno.js.cn/release/v1.7.0/deno-x86_64-apple-darwin.zip",
        "https://dl.deno.land/release/v1.7.0/deno-aarch64-apple-darwin.zip",
        "https://dl.deno.js.cn/release/v1.7.0/deno-aarch64-apple-darwin.zip"
      );
    } else if #[cfg(target_os = "linux")] {
      asserts_eq_one_of!(
        url.as_str(),
        "https://dl.deno.land/release/v1.7.0/deno-x86_64-unknown-linux-gnu.zip",
        "https://dl.deno.js.cn/release/v1.7.0/deno-x86_64-unknown-linux-gnu.zip"
      );
    }
  }
}
