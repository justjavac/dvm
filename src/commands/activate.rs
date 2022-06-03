use crate::commands::use_version;
use crate::utils::check_is_deactivated;
use crate::{dvm_root, DvmMeta};
use anyhow::Result;

pub fn exec(meta: &mut DvmMeta) -> Result<()> {
  let home = dvm_root();
  if check_is_deactivated() {
    std::fs::remove_file(home.join(".deactivated")).unwrap();
  }

  use_version::exec(meta, None, false)
}
