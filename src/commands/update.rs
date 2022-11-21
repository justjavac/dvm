use crate::version::cache_remote_versions;
use crate::DvmMeta;
use anyhow::Result;

pub fn exec(_meta: &mut DvmMeta) -> Result<()> {
  cache_remote_versions()?;
  Ok(())
}
