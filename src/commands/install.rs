// Copyright 2018-2020 the Deno authors. All rights reserved. MIT license.
// Copyright 2020-2021 justjavac. All rights reserved. MIT license.
use anyhow::Result;
use semver_parser::version::{parse as semver_parse, Version};
use ureq::{Agent, AgentBuilder, Error};

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
  let agent: Agent = AgentBuilder::new().build();

  let install_version = match version {
    Some(passed_version) => match semver_parse(&passed_version) {
      Ok(ver) => ver,
      Err(_) => {
        eprintln!("Invalid semver");
        std::process::exit(1)
      }
    },
    None => get_latest_version(&agent)?,
  };

  let exe_path = get_exe_path(&install_version);

  if exe_path.exists() {
    println!("version v{} is already installed", install_version);
  } else {
    let archive_data = download_package(
      &compose_url_to_exec(&install_version),
      agent,
      &install_version,
    )?;
    unpack(archive_data, &install_version)?;
  }

  if !no_use {
    use_::use_this_bin_path(&exe_path, &install_version)?;
  }

  Ok(())
}

fn get_latest_version(agent: &Agent) -> Result<Version> {
  println!("Checking for latest version");
  let body = agent
    .get("https://dl.deno.land/release-latest.txt")
    .call()?
    .into_string()?;
  let v = body.trim().replace("v", "");
  println!("The latest version is v{}", &v);
  Ok(semver_parse(&v).unwrap())
}

fn download_package(
  url: &str,
  agent: Agent,
  version: &Version,
) -> Result<Vec<u8>> {
  println!("downloading {}", &url);

  let response = match agent.get(url).call() {
    Ok(response) => response,
    Err(Error::Status(code, _response)) => {
      println!("Request error with code: {}", code);
      std::process::exit(1)
    }
    Err(error) => {
      println!("Network error {}", &error);
      std::process::exit(1)
    }
  };

  if response.status() == 404 {
    println!("Version has not been found, aborting");
    std::process::exit(1)
  }

  if response.status() >= 400 && response.status() <= 599 {
    println!("Download '{}' failed: {}", &url, response.status());
    std::process::exit(1)
  }

  println!("Version has been found");
  println!("Deno v{} has been downloaded", &version);

  let mut buf: Vec<u8> = vec![];
  response.into_reader().read_to_end(&mut buf)?;
  Ok(buf)
}

fn compose_url_to_exec(version: &Version) -> String {
  if version.major >= 1 && version.minor >= 7 {
    format!("https://dl.deno.land/release/v{}/{}", version, ARCHIVE_NAME)
  } else {
    format!(
      "https://cdn.jsdelivr.net/gh/justjavac/deno_releases/{}/{}",
      version, ARCHIVE_NAME
    )
  }
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
fn test_compose_url_to_exec_lte_1_7() {
  let v = semver_parse("0.0.1").unwrap();
  let url = compose_url_to_exec(&v);
  #[cfg(windows)]
  assert_eq!(url.as_str(), "https://cdn.jsdelivr.net/gh/justjavac/deno_releases/0.0.1/deno-x86_64-pc-windows-msvc.zip");
  #[cfg(target_os = "macos")]
  assert_eq!(
    url.as_str(),
    "https://cdn.jsdelivr.net/gh/justjavac/deno_releases/0.0.1/deno-x86_64-apple-darwin.zip"
  );
  #[cfg(target_os = "linux")]
  assert_eq!(url.as_str(), "https://cdn.jsdelivr.net/gh/justjavac/deno_releases/0.0.1/deno-x86_64-unknown-linux-gnu.zip");
}

#[test]
fn test_compose_url_to_exec() {
  let v = semver_parse("1.7.0").unwrap();
  let url = compose_url_to_exec(&v);
  #[cfg(windows)]
  assert_eq!(
    url.as_str(),
    "https://dl.deno.land/release/v1.7.0/deno-x86_64-pc-windows-msvc.zip"
  );
  #[cfg(target_os = "macos")]
  assert_eq!(
    url.as_str(),
    "https://dl.deno.land/release/v1.7.0/deno-x86_64-apple-darwin.zip"
  );
  #[cfg(target_os = "linux")]
  assert_eq!(
    url.as_str(),
    "https://dl.deno.land/release/v1.7.0/deno-x86_64-unknown-linux-gnu.zip"
  );
}
