// Copyright 2018-2020 the Deno authors. All rights reserved. MIT license.
// Copyright 2020-2022 justjavac. All rights reserved. MIT license.
use super::use_version;
use crate::configrc::rc_get_with_fix;
use crate::consts::{
  DVM_CACHE_PATH_PREFIX, DVM_CANARY_PATH_PREFIX, DVM_CONFIGRC_KEY_REGISTRY_BINARY, DVM_CONFIGRC_KEY_REGISTRY_VERSION,
  DVM_VERSION_CANARY, DVM_VERSION_LATEST, DVM_VERSION_LTS, REGISTRY_LIST_OFFICIAL, REGISTRY_OFFICIAL,
};
use crate::meta::DvmMeta;
use crate::utils::{deno_canary_path, deno_version_path, dvm_root, print_error};
use crate::version::{get_latest_canary, get_latest_lts_version, get_latest_remote_version};
use anyhow::Result;
use cfg_if::cfg_if;
use semver::Version;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

cfg_if! {
  if #[cfg(windows)] {
    const ARCHIVE_NAME: &str = "deno-x86_64-pc-windows-msvc.zip";
  } else if #[cfg(all(target_os = "macos", target_arch = "aarch64"))] {
    const ARCHIVE_NAME: &str = "deno-aarch64-apple-darwin.zip";
  } else if #[cfg(all(target_os = "macos", target_arch = "x86_64"))] {
    const ARCHIVE_NAME: &str = "deno-x86_64-apple-darwin.zip";
  } else if #[cfg(all(target_os = "linux", target_arch = "x86_64"))] {
    const ARCHIVE_NAME: &str = "deno-x86_64-unknown-linux-gnu.zip";
  } else if #[cfg(all(target_os = "linux", target_arch = "aarch64"))] {
    const ARCHIVE_NAME: &str = "deno-aarch64-unknown-linux-gnu.zip";
  } else {
    compile_error!("dvm does not know which Deno archive to download for this target");
  }
}

pub fn exec(_: &DvmMeta, no_use: bool, version: Option<String>) -> Result<()> {
  let binary_registry_url =
    rc_get_with_fix(DVM_CONFIGRC_KEY_REGISTRY_BINARY).unwrap_or_else(|_| REGISTRY_OFFICIAL.to_string());
  let version_registry_url =
    rc_get_with_fix(DVM_CONFIGRC_KEY_REGISTRY_VERSION).unwrap_or_else(|_| REGISTRY_LIST_OFFICIAL.to_string());

  if version.as_deref() == Some(DVM_VERSION_CANARY) {
    let hash = get_latest_canary(&binary_registry_url)?;
    download_and_unpack_canary(&binary_registry_url, &hash)?;

    if !no_use {
      use_version::use_canary_bin_path(false)?;
    }

    return Ok(());
  }

  let install_version = match version {
    Some(ref passed_version) if passed_version == DVM_VERSION_LATEST => {
      println!("Checking for latest version");
      let version = get_latest_remote_version(&version_registry_url)?;
      println!("The latest version is v{}", version);
      version
    }
    Some(ref passed_version) if passed_version == DVM_VERSION_LTS => {
      println!("Checking for latest LTS version");
      let version = get_latest_lts_version()?;
      println!("The latest LTS version is v{}", version);
      version
    }
    Some(ref passed_version) => {
      Version::parse(passed_version).map_err(|_| anyhow::format_err!("Invalid semver {}", passed_version))?
    }
    None => {
      println!("Checking for latest version");
      let version = get_latest_remote_version(&version_registry_url)?;
      println!("The latest version is v{}", version);
      version
    }
  };

  let exe_path = deno_version_path(&install_version);

  if exe_path.exists() {
    println!("Version v{} is already installed", install_version);
  } else {
    download_and_unpack_package(
      &compose_url_to_exec(&binary_registry_url, &install_version),
      &install_version,
    )?;
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

/// Fetch `url`, rejecting error responses so a 404 page never reaches the
/// unpacker as if it were an archive.
fn download_archive(url: &str) -> Result<Vec<u8>> {
  println!("downloading {}", url);

  let response = match ureq::get(url).timeout(Duration::from_secs(30)).call() {
    Ok(response) => response,
    Err(ureq::Error::Status(404, _)) => anyhow::bail!("'{}' has not been found", url),
    Err(ureq::Error::Status(code, _)) => anyhow::bail!("Download '{}' failed: {}", url, code),
    Err(e) => anyhow::bail!("Network error {}", e),
  };

  let mut bytes = Vec::new();
  response.into_reader().read_to_end(&mut bytes)?;
  Ok(bytes)
}

/// Download a .sha256 checksum file and return the hex-encoded hash.
/// Handles the standard `sha256sum` format: `<hash>  <filename>`.
fn download_sha256(url: &str) -> Result<String> {
  let content = download_archive(url)?;
  let text = String::from_utf8(content)?;
  // sha256sum format: "HASH  FILENAME" or just "HASH"
  let hash = text
    .lines()
    .next()
    .and_then(|line| line.split_whitespace().next())
    .ok_or_else(|| anyhow::anyhow!("Empty checksum file"))?
    .to_string();

  if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
    anyhow::bail!("Invalid SHA256 checksum format");
  }

  Ok(hash.to_lowercase())
}

fn compose_url_to_exec(registry: &str, version: &Version) -> String {
  format!("{}release/v{}/{}", registry, version, ARCHIVE_NAME)
}

fn download_and_unpack_package(url: &str, version: &Version) -> Result<()> {
  let archive_data = download_archive(url)?;
  println!("Version has been found");
  println!("Deno v{} has been downloaded", version);

  // Best-effort checksum verification.  If the checksum file is unavailable
  // (e.g. on a custom mirror that doesn't publish .sha256 files), we log a
  // warning and continue rather than hard-failing.
  let checksum_url = format!("{}.sha256", url);
  match download_sha256(&checksum_url) {
    Ok(expected) => {
      let actual = format!("{:x}", Sha256::digest(&archive_data));
      if actual != expected {
        anyhow::bail!(
          "Checksum mismatch for {}:\n  expected: {}\n  actual:   {}",
          ARCHIVE_NAME,
          expected,
          actual
        );
      }
      println!("Checksum verified OK");
    }
    Err(err) => {
      eprintln!("Warning: could not verify checksum: {}", err);
    }
  }

  if let Err(err) = unpack(archive_data, version) {
    print_error(&format!("Failed to unpack Deno v{}: {}", version, err));
    eprintln!("Removing the corrupted archive and retrying download");
    remove_version_dir(version)?;

    let archive_data = download_archive(url)?;
    if let Err(retry_err) = unpack(archive_data, version) {
      remove_version_dir(version)?;
      return Err(anyhow::anyhow!(
        "Failed to unpack Deno v{} after retry: {}",
        version,
        retry_err
      ));
    }
  }

  Ok(())
}

fn remove_version_dir(version: &Version) -> Result<()> {
  let version_dir = dvm_root().join(format!("{}/{}", DVM_CACHE_PATH_PREFIX, version));
  if version_dir.exists() {
    fs::remove_dir_all(version_dir)?;
  }
  Ok(())
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
    fs::remove_file(&exe_path)?;
  }

  unpack_impl(archive_data, canary_dir, exe_path)
}

fn unpack_impl(archive_data: Vec<u8>, version_dir: PathBuf, path: PathBuf) -> Result<PathBuf> {
  let archive_ext = Path::new(ARCHIVE_NAME)
    .extension()
    .and_then(|ext| ext.to_str())
    .expect("ARCHIVE_NAME always has a UTF-8 extension");

  match archive_ext {
    "zip" => unpack_zip(&archive_data, &version_dir)?,
    ext => anyhow::bail!("Unsupported archive type: '{}'", ext),
  }

  if !path.exists() {
    anyhow::bail!("Unpacked archive did not contain {}", path.display());
  }
  Ok(version_dir)
}

fn unpack_zip(archive_data: &[u8], dest_dir: &Path) -> Result<()> {
  let reader = io::Cursor::new(archive_data);
  let mut zip = zip::ZipArchive::new(reader)?;

  for i in 0..zip.len() {
    let mut file = zip.by_index(i)?;
    let out_path = dest_dir.join(file.name());

    // Sanity check: prevent zip-slip path traversal.
    // We check both that the resolved path stays within dest_dir
    // AND that no individual component is `..` (which would bypass
    // a naive starts_with check since Path::starts_with compares
    // components without resolving `..`).
    let name = file.name();
    let has_parent_component = Path::new(name)
      .components()
      .any(|c| matches!(c, std::path::Component::ParentDir));
    if has_parent_component || !out_path.starts_with(dest_dir) {
      anyhow::bail!("Invalid zip entry path: {}", file.name());
    }

    if file.is_dir() {
      fs::create_dir_all(&out_path)?;
    } else if file.unix_mode().is_some_and(|mode| mode & 0o170000 == 0o120000) {
      // Symlink entry detected via Unix mode (S_IFLNK = 0o120000)
      // Read the symlink target from the entry content
      let mut target = String::new();
      io::Read::read_to_string(&mut file, &mut target)?;
      let target = target.trim();

      // Validate the symlink target doesn't escape the destination directory.
      // Absolute targets always escape. For relative targets, resolve them
      // (without touching the filesystem) and verify they stay within dest_dir.
      let target_path = Path::new(target);
      if target_path.is_absolute() {
        anyhow::bail!(
          "Symlink target is absolute, refusing to extract: {} -> {}",
          file.name(),
          target
        );
      }
      let symlink_dir = out_path.parent().unwrap_or(dest_dir);
      let mut components = Vec::new();
      for component in target_path.components() {
        use std::path::Component;
        match component {
          Component::ParentDir => {
            components.pop();
          }
          Component::Normal(part) => {
            components.push(part.to_os_string());
          }
          Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
      }
      let resolved = symlink_dir.join(PathBuf::from_iter(&components));
      if !resolved.starts_with(dest_dir) {
        anyhow::bail!(
          "Symlink target escapes destination: {} -> {}",
          file.name(),
          target
        );
      }

      if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
      }

      cfg_if! {
        if #[cfg(unix)] {
          use std::os::unix::fs::symlink;
          // Remove existing file/symlink if present
          if out_path.symlink_metadata().is_ok() {
            let _ = fs::remove_file(&out_path);
          }
          symlink(target, &out_path)?;
        } else if #[cfg(windows)] {
          // On Windows, creating symlinks requires admin privileges.
          // Warn and skip rather than silently extracting as a text file.
          eprintln!(
            "Warning: skipping symlink entry {} -> {} (Windows symlinks require admin privileges)",
            file.name(),
            target
          );
        } else {
          eprintln!(
            "Warning: skipping symlink entry {} -> {} (unsupported platform)",
            file.name(),
            target
          );
        }
      }
    } else {
      if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent)?;
      }
      let mut out_file = fs::File::create(&out_path)?;
      io::copy(&mut file, &mut out_file)?;

      // Preserve Unix permissions (executable bit)
      #[cfg(unix)]
      {
        use std::os::unix::fs::PermissionsExt;
        if let Some(mode) = file.unix_mode() {
          fs::set_permissions(&out_path, fs::Permissions::from_mode(mode))?;
        }
      }
    }
  }

  Ok(())
}

fn compose_url_to_canary(registry: &str, hash: &str) -> String {
  format!("{}canary/{}/{}", registry, hash, ARCHIVE_NAME)
}

/// Same retry as `download_and_unpack_package`: a truncated download would
/// otherwise leave a permanently broken canary behind (see
/// <https://github.com/justjavac/dvm/issues/242>).
fn download_and_unpack_canary(registry: &str, hash: &str) -> Result<()> {
  let url = compose_url_to_canary(registry, hash);

  let archive_data = download_archive(&url)?;

  // Best-effort checksum verification.  If the checksum file is unavailable
  // (e.g. on a custom mirror that doesn't publish .sha256 files), we log a
  // warning and continue rather than hard-failing.
  let checksum_url = format!("{}.sha256", url);
  match download_sha256(&checksum_url) {
    Ok(expected) => {
      let actual = format!("{:x}", Sha256::digest(&archive_data));
      if actual != expected {
        anyhow::bail!(
          "Checksum mismatch for {}:\n  expected: {}\n  actual:   {}",
          ARCHIVE_NAME,
          expected,
          actual
        );
      }
      println!("Checksum verified OK");
    }
    Err(err) => {
      eprintln!("Warning: could not verify checksum: {}", err);
    }
  }

  if let Err(err) = unpack_canary(archive_data) {
    print_error(&format!("Failed to unpack Deno canary {}: {}", hash, err));
    eprintln!("Removing the corrupted archive and retrying download");
    remove_canary_dir()?;

    let archive_data = download_archive(&url)?;
    if let Err(retry_err) = unpack_canary(archive_data) {
      remove_canary_dir()?;
      return Err(anyhow::anyhow!(
        "Failed to unpack Deno canary {} after retry: {}",
        hash,
        retry_err
      ));
    }
  }

  Ok(())
}

fn remove_canary_dir() -> Result<()> {
  let canary_dir = dvm_root().join(DVM_CANARY_PATH_PREFIX);
  if canary_dir.exists() {
    fs::remove_dir_all(canary_dir)?;
  }
  Ok(())
}

#[cfg(test)]
mod unpack_zip_tests {
  use super::*;
  use std::io::Write;
  use tempfile::tempdir;
  use zip::write::FileOptions;
  use zip::CompressionMethod;
  use zip::ZipWriter;

  /// A test zip entry: either a file with content, or a directory.
  enum ZipEntry<'a> {
    File(&'a str, &'a [u8], FileOptions),
    Dir(&'a str, FileOptions),
  }

  /// Helper: create a zip archive in memory.
  fn create_test_zip(entries: &[ZipEntry]) -> Vec<u8> {
    let mut buf = Vec::new();
    {
      let mut zw = ZipWriter::new(std::io::Cursor::new(&mut buf));
      for entry in entries {
        match entry {
          ZipEntry::File(name, content, options) => {
            zw.start_file(name.to_string(), options.clone()).unwrap();
            zw.write_all(content).unwrap();
          }
          ZipEntry::Dir(name, options) => {
            zw.add_directory(name.to_string(), options.clone()).unwrap();
          }
        }
      }
      zw.finish().unwrap();
    }
    buf
  }

  #[test]
  fn test_basic_file_extraction() {
    let zip_data = create_test_zip(&[ZipEntry::File(
      "hello.txt",
      b"Hello, World!",
      FileOptions::default(),
    )]);

    let tmp = tempdir().unwrap();
    unpack_zip(&zip_data, tmp.path()).unwrap();

    let extracted = std::fs::read_to_string(tmp.path().join("hello.txt")).unwrap();
    assert_eq!(extracted, "Hello, World!");
  }

  #[test]
  fn test_zip_slip_protection() {
    let zip_data = create_test_zip(&[ZipEntry::File(
      "../../../etc/passwd",
      b"root:x:0:0:root:/root:/bin/bash",
      FileOptions::default(),
    )]);

    let tmp = tempdir().unwrap();
    let result = unpack_zip(&zip_data, tmp.path());
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(
      err_msg.contains("Invalid zip entry path"),
      "expected zip-slip error, got: {}",
      err_msg
    );
  }

  #[test]
  fn test_directory_creation() {
    let dir_opts = FileOptions::default().unix_permissions(0o755);
    let file_opts = FileOptions::default();
    let zip_data = create_test_zip(&[
      ZipEntry::Dir("dir1", dir_opts.clone()),
      ZipEntry::Dir("dir1/subdir", dir_opts.clone()),
      ZipEntry::File("dir1/subdir/file.txt", b"nested content", file_opts),
    ]);

    let tmp = tempdir().unwrap();
    unpack_zip(&zip_data, tmp.path()).unwrap();

    assert!(tmp.path().join("dir1").is_dir());
    assert!(tmp.path().join("dir1/subdir").is_dir());
    assert!(tmp.path().join("dir1/subdir/file.txt").is_file());

    let content = std::fs::read_to_string(tmp.path().join("dir1/subdir/file.txt")).unwrap();
    assert_eq!(content, "nested content");
  }

  #[test]
  fn test_empty_zip() {
    let zip_data = create_test_zip(&[]);

    let tmp = tempdir().unwrap();
    let result = unpack_zip(&zip_data, tmp.path());
    assert!(result.is_ok());

    // Dest dir should still exist
    assert!(tmp.path().exists());
  }

  #[test]
  fn test_multiple_files() {
    let zip_data = create_test_zip(&[
      ZipEntry::File("file1.txt", b"content 1", FileOptions::default()),
      ZipEntry::File("file2.txt", b"content 2", FileOptions::default()),
      ZipEntry::File("file3.txt", b"content 3", FileOptions::default()),
    ]);

    let tmp = tempdir().unwrap();
    unpack_zip(&zip_data, tmp.path()).unwrap();

    assert_eq!(
      std::fs::read_to_string(tmp.path().join("file1.txt")).unwrap(),
      "content 1"
    );
    assert_eq!(
      std::fs::read_to_string(tmp.path().join("file2.txt")).unwrap(),
      "content 2"
    );
    assert_eq!(
      std::fs::read_to_string(tmp.path().join("file3.txt")).unwrap(),
      "content 3"
    );
  }

  #[cfg(unix)]
  #[test]
  fn test_unix_permission_preservation() {
    use std::os::unix::fs::PermissionsExt;

    let zip_data = create_test_zip(&[ZipEntry::File(
      "executable.sh",
      b"#!/bin/sh\necho hello",
      FileOptions::default().unix_permissions(0o755),
    )]);

    let tmp = tempdir().unwrap();
    unpack_zip(&zip_data, tmp.path()).unwrap();

    let metadata = std::fs::metadata(tmp.path().join("executable.sh")).unwrap();
    let mode = metadata.permissions().mode();
    // Check that the executable bit is set (owner/group/other execute)
    assert!(mode & 0o111 != 0, "expected executable permission, got mode: {:o}", mode);
  }

  #[test]
  fn test_nested_dir_without_explicit_dir_entries() {
    // Some zips don't have explicit directory entries — dirs are implied by
    // file paths. unpack_zip should still create parent directories.
    let zip_data = create_test_zip(&[ZipEntry::File(
      "a/b/c/deep.txt",
      b"deep file",
      FileOptions::default(),
    )]);

    let tmp = tempdir().unwrap();
    unpack_zip(&zip_data, tmp.path()).unwrap();

    assert!(tmp.path().join("a/b/c").is_dir());
    assert_eq!(
      std::fs::read_to_string(tmp.path().join("a/b/c/deep.txt")).unwrap(),
      "deep file"
    );
  }

  #[test]
  fn test_symlink_entry_is_handled() {
    // Create a zip with a symlink entry using the raw API.
    // The current unpack_zip doesn't explicitly handle symlinks — it treats
    // them as regular files (writes the link target as content).
    // This test documents the current behavior and ensures it doesn't crash.
    let zip_data = {
      let mut buf = Vec::new();
      {
        let mut zw = ZipWriter::new(std::io::Cursor::new(&mut buf));
        // Write a regular file first
        zw.start_file("target.txt", FileOptions::default()).unwrap();
        zw.write_all(b"target content").unwrap();
        // Write a symlink entry using add_symlink (if available)
        // zip 0.6 supports symlink via unix_permissions with 0o120000 type
        // We use start_file with symlink permissions — the zip crate handles it
        zw.start_file(
          "link.txt",
          FileOptions::default()
            .unix_permissions(0o120777)
            .compression_method(CompressionMethod::Stored),
        )
        .unwrap();
        zw.write_all(b"target.txt").unwrap();
        zw.finish().unwrap();
      }
      buf
    };

    let tmp = tempdir().unwrap();
    // Should not crash — symlink entries are treated as regular files
    let result = unpack_zip(&zip_data, tmp.path());
    assert!(result.is_ok());

    // The "symlink" should exist as a regular file containing the link target text
    let link_path = tmp.path().join("link.txt");
    assert!(link_path.is_file());
    assert_eq!(std::fs::read_to_string(&link_path).unwrap(), "target.txt");
  }
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
    } else if #[cfg(all(target_os = "macos", target_arch = "x86_64"))] {
      asserts_eq_one_of!(
        url.as_str(),
        "https://dl.deno.land/release/v1.7.0/deno-x86_64-apple-darwin.zip",
        "https://dl.deno.js.cn/release/v1.7.0/deno-x86_64-apple-darwin.zip"
      );
    } else if #[cfg(all(target_os = "macos", target_arch = "aarch64"))] {
      asserts_eq_one_of!(
        url.as_str(),
        "https://dl.deno.land/release/v1.7.0/deno-aarch64-apple-darwin.zip",
        "https://dl.deno.js.cn/release/v1.7.0/deno-aarch64-apple-darwin.zip"
      );
    } else if #[cfg(all(target_os = "linux", target_arch = "x86_64"))] {
      asserts_eq_one_of!(
        url.as_str(),
        "https://dl.deno.land/release/v1.7.0/deno-x86_64-unknown-linux-gnu.zip",
        "https://dl.deno.js.cn/release/v1.7.0/deno-x86_64-unknown-linux-gnu.zip"
      );
    } else if #[cfg(all(target_os = "linux", target_arch = "aarch64"))] {
      asserts_eq_one_of!(
        url.as_str(),
        "https://dl.deno.land/release/v1.7.0/deno-aarch64-unknown-linux-gnu.zip",
        "https://dl.deno.js.cn/release/v1.7.0/deno-aarch64-unknown-linux-gnu.zip"
      );
    }
  }
}
