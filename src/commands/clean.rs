use crate::configrc::rc_clean;
use crate::{dvm_root, DvmMeta};
use anyhow::Result;
use std::io::{stdin, stdout, BufReader, Read, Write};

pub fn exec(meta: &mut DvmMeta, yes: bool) -> Result<()> {
  let home = dvm_root();

  let cache_folder = home.join("versions");
  if !cache_folder.exists() {
    println!("Nothing to clean");
    return Ok(());
  }

  if !yes && !prompt_confirm("Are you sure you want to clean dvm cache? (y/N)") {
    println!("Clean cancelled");
    return Ok(());
  }

  let requires = meta
    .versions
    .iter()
    .filter_map(|v| {
      if v.is_valid_mapping() {
        None
      } else {
        Some(v.required.clone())
      }
    })
    .collect::<Vec<_>>();

  for required in requires {
    meta.delete_version_mapping(required.clone())?;
  }

  meta.clean_files();

  // clean user-wide rc file
  rc_clean(true)?;
  rc_clean(false)?;

  println!("Cleaned successfully");
  Ok(())
}

/// Prompt the user for confirmation, defaulting to "no".
/// Returns true only if the user explicitly confirms with "y" or "Y".
fn prompt_confirm(prompt: &str) -> bool {
  print!("{} ", prompt);

  let _ = stdout().flush();
  let mut buffer = [0; 1];
  let confirm = BufReader::new(stdin())
    .read(&mut buffer)
    .ok()
    .map(|_| buffer[0] as char)
    .unwrap_or_else(|| 'n');
  confirm.eq_ignore_ascii_case(&'y')
}
