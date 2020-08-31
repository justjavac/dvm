// Copyright 2018-2020 the Deno authors. All rights reserved. MIT license.
// Copyright 2020 justjavac. All rights reserved. MIT license.
use anyhow::{anyhow, Result};
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::StatusCode;
use semver_parser::version::{parse as semver_parse, Version};
use url::Url;

use std::fs;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::string::String;

use super::use_;
use crate::utils::get_dvm_root;
use crate::utils::get_exe_path;

#[cfg(windows)]
const ARCHIVE_NAME: &str = "deno-x86_64-pc-windows-msvc.zip";
#[cfg(target_os = "macos")]
const ARCHIVE_NAME: &str = "deno-x86_64-apple-darwin.zip";
#[cfg(target_os = "linux")]
const ARCHIVE_NAME: &str = "deno-x86_64-unknown-linux-gnu.zip";

pub fn exec(no_use: bool, version: Option<String>) -> Result<()> {
  let client_builder = Client::builder();
  let client = client_builder.build()?;

  let install_version = match version {
    Some(passed_version) => match semver_parse(&passed_version) {
      Ok(ver) => ver,
      Err(_) => {
        eprintln!("Invalid semver");
        std::process::exit(1)
      }
    },
    None => get_latest_version(&client)?,
  };

  let exe_path = get_exe_path(&install_version);

  if exe_path.exists() {
    println!("version v{} is already installed", install_version);
  } else {
    let archive_data = download_package(
      &compose_url_to_exec(&install_version)?,
      client,
      &install_version,
    )?;
    unpack(archive_data, &install_version)?;
  }

  if !no_use {
    use_::use_this_bin_path(&exe_path, &install_version)?;
  }

  Ok(())
}

fn get_latest_version(client: &Client) -> Result<Version> {
  println!("Checking for latest version");
  let body = client
    .get(Url::parse(
      "https://github.com/denoland/deno/releases/latest",
    )?)
    .send()?
    .text()?;
  let v = find_version(&body)?;
  println!("The latest version is {}", &v);
  Ok(semver_parse(&v).unwrap())
}

fn download_package(
  url: &Url,
  client: Client,
  version: &Version,
) -> Result<Vec<u8>> {
  println!("downloading {}", url);
  let url = url.clone();
  let version = version.clone();

  let mut response = match client.get(url.clone()).send() {
    Ok(response) => response,
    Err(error) => {
      println!("Network error {}", &error);
      std::process::exit(1)
    }
  };

  if response.status().is_success() {
    println!("Version has been found");
    println!("Deno v{} has been downloaded", &version);
  }

  if response.status() == StatusCode::NOT_FOUND {
    println!("Version has not been found, aborting");
    std::process::exit(1)
  }

  if response.status().is_client_error() || response.status().is_server_error()
  {
    println!("Download '{}' failed: {}", &url, response.status());
    std::process::exit(1)
  }

  let mut buf: Vec<u8> = vec![];
  response.copy_to(&mut buf)?;
  Ok(buf)
}

fn compose_url_to_exec(version: &Version) -> Result<Url> {
  let s = format!(
    "https://github.com/denoland/deno/releases/download/v{}/{}",
    version, ARCHIVE_NAME
  );
  Ok(Url::parse(&s)?)
}

fn find_version(text: &str) -> Result<String> {
  let re = Regex::new(r#"v(\d+\.\d+\.\d+) "#)?;
  if let Some(_mat) = re.find(text) {
    let mat = _mat.as_str();
    return Ok(mat[1..mat.len() - 1].to_string());
  }
  Err(anyhow!("Cannot read latest tag version"))
}

fn unpack(archive_data: Vec<u8>, version: &Version) -> Result<PathBuf> {
  let dvm_dir = get_dvm_root().join(format!("{}", version));
  fs::create_dir_all(&dvm_dir)?;
  let exe_path = get_exe_path(version);

  let archive_ext = Path::new(ARCHIVE_NAME)
    .extension()
    .and_then(|ext| ext.to_str())
    .unwrap();
  let unpack_status = match archive_ext {
    "gz" => {
      let exe_file = fs::File::create(&exe_path)?;
      let mut cmd = Command::new("gunzip")
        .arg("-c")
        .stdin(Stdio::piped())
        .stdout(Stdio::from(exe_file))
        .spawn()?;
      cmd.stdin.as_mut().unwrap().write_all(&archive_data)?;
      cmd.wait()?
    }
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
  let v = semver_parse("0.0.1").unwrap();
  let url = compose_url_to_exec(&v).unwrap();
  #[cfg(windows)]
  assert_eq!(url.as_str(), "https://github.com/denoland/deno/releases/download/v0.0.1/deno-x86_64-pc-windows-msvc.zip");
  #[cfg(target_os = "macos")]
  assert_eq!(
    url.as_str(),
    "https://github.com/denoland/deno/releases/download/v0.0.1/deno-x86_64-apple-darwin.zip"
  );
  #[cfg(target_os = "linux")]
  assert_eq!(url.as_str(), "https://github.com/denoland/deno/releases/download/v0.0.1/deno-x86_64-unknown-linux-gnu.zip");
}
