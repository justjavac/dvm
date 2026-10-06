// Copyright 2018-2020 the Deno authors. All rights reserved. MIT license.
// Copyright 2020-2022 justjavac. All rights reserved. MIT license.
use anyhow::Result;
use sha2::{Digest, Sha256};

/// Fetch `url` via HTTP and return the response body as bytes.
/// Rejects error responses so a 404 page never reaches the caller as if
/// it were valid data.
pub fn download_bytes(url: &str) -> Result<Vec<u8>> {
  println!("downloading {}", url);

  let response = match tinyget::get(url).send() {
    Ok(response) => response,
    Err(error) => anyhow::bail!("Network error {}", error),
  };

  if response.status_code == 404 {
    anyhow::bail!("'{}' has not been found", url);
  }

  if response.status_code >= 400 {
    anyhow::bail!("Download '{}' failed: {}", url, response.status_code);
  }

  Ok(response.into_bytes())
}

/// Download a .sha256 checksum file and return the hex-encoded hash.
/// Handles the standard `sha256sum` format: `<hash>  <filename>`.
pub fn download_sha256(url: &str) -> Result<String> {
  let content = download_bytes(url)?;
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

/// Download an archive and verify its checksum (best-effort).
///
/// Downloads the archive from `archive_url`, then attempts to download
/// `archive_url + ".sha256"` and verify the checksum.  If the checksum
/// file is unavailable (e.g. on a custom mirror that doesn't publish
/// .sha256 files), a warning is printed and the download proceeds anyway.
pub fn download_with_checksum(archive_url: &str, archive_name: &str) -> Result<Vec<u8>> {
  let archive_data = download_bytes(archive_url)?;

  // Best-effort checksum verification.  If the checksum file is unavailable
  // (e.g. on a custom mirror that doesn't publish .sha256 files), we log a
  // warning and continue rather than hard-failing.
  let checksum_url = format!("{}.sha256", archive_url);
  match download_sha256(&checksum_url) {
    Ok(expected) => {
      let actual = format!("{:x}", Sha256::digest(&archive_data));
      if actual != expected {
        anyhow::bail!(
          "Checksum mismatch for {}:\n  expected: {}\n  actual:   {}",
          archive_name,
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

  Ok(archive_data)
}
