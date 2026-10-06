use crate::utils::deno_bin_path;
use anyhow::Result;

/// Print the path to the current deno executable.
///
/// Similar to `nvm which`, `rbenv which`, and `pyenv which`.
pub fn exec() -> Result<()> {
  let bin_path = deno_bin_path();

  if bin_path.exists() {
    println!("{}", bin_path.display());
    Ok(())
  } else {
    anyhow::bail!(
      "no deno version is currently active via dvm.\n\
       Run `dvm use <version>` to select a version, or `dvm install` to install one."
    )
  }
}
