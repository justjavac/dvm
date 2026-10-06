use crate::utils::print_warning;
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::time::Duration;

/// Core download function: fetches `url` bytes with error handling.
/// No user-facing output — use `download_with_checksum` for archives.
fn download_bytes(url: &str) -> Result<Vec<u8>> {
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

/// Download an archive and verify its checksum.
///
/// Fetches the archive from `archive_url`, then attempts to download
/// `archive_url + ".sha256"` and verify the checksum. Best-effort: if the
/// checksum file is unavailable (e.g. on a custom mirror that doesn't publish
/// `.sha256` files), a warning is printed and the download proceeds anyway.
pub fn download_with_checksum(archive_url: &str, archive_name: &str) -> Result<Vec<u8>> {
  println!("downloading {}", archive_url);
  let archive_data = download_bytes(archive_url)?;

  let checksum_url = format!("{}.sha256", archive_url);
  match download_sha256(&checksum_url) {
    Ok(expected) => {
      let actual = format!("{:x}", Sha256::digest(&archive_data));
      if actual != expected {
        anyhow::bail!(
          "Checksum verification failed for {}.\nThis usually means the download was corrupted or tampered with.\nExpected: {}\nActual:   {}\nTry running `dvm install` again, or switch to a different registry.",
          archive_name,
          expected,
          actual
        );
      }
      println!("Checksum verified OK");
    }
    Err(err) => {
      print_warning(&format!("could not verify checksum: {}", err));
    }
  }

  Ok(archive_data)
}
