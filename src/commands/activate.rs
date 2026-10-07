use crate::commands::use_version;
use crate::utils::check_is_deactivated;
use crate::{dvm_root, DvmMeta};
use anyhow::Result;

pub fn exec(meta: &mut DvmMeta) -> Result<()> {
  // Perform the actual activation first (setting up the deno binary link,
  // updating config, etc.). Only after that succeeds do we remove the
  // deactivation mark file. This ensures that if activation fails partway
  // through, the system stays in a consistent state: the deactivation mark
  // remains, so dvm still appears deactivated (which is accurate since the
  // activation didn't complete).
  use_version::exec(meta, None, false)?;

  let home = dvm_root();
  if check_is_deactivated() {
    std::fs::remove_file(home.join(".deactivated"))?;
  }

  Ok(())
}
