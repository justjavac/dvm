use crate::utils::check_is_deactivated;
use crate::{deno_bin_path, dvm_root, DvmMeta};
use anyhow::{anyhow, Result};

pub fn exec(meta: &mut DvmMeta) -> Result<()> {
  let home = dvm_root();
  if !check_is_deactivated() {
    std::fs::write(home.join(".deactivated"), "").unwrap();
  }

  std::fs::remove_file(deno_bin_path())
    .map(|_| ())
    .map_err(|e| anyhow!(e))
}
