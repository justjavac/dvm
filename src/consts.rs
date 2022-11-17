pub const REGISTRY_OFFICIAL: &str = "https://dl.deno.land/";
pub const REGISTRY_CN: &str = "https://dl.deno.js.cn/";
pub const REGISTRY_LIST_OFFICIAL: &str = "https://raw.githubusercontent.com/denoland/dotland/main/versions.json";
pub const REGISTRY_LIST_CN: &str = "https://dl.deno.js.cn/versions.json";

pub const REGISTRY_LATEST_RELEASE_PATH: &str = "release-latest.txt";
pub const REGISTRY_LATEST_CANARY_PATH: &str = "canary-latest.txt";
pub const REGISTRY_NAME_CN: &str = "cn";
pub const REGISTRY_NAME_OFFICIAL: &str = "official";

pub const DVM_CACHE_PATH_PREFIX: &str = "versions";
#[allow(dead_code)]
pub const DVM_CACHE_REMOTE_PATH: &str = "cached-remote-versions.json";
pub const DVM_CANARY_PATH_PREFIX: &str = "canary";
pub const DVM_CACHE_INVALID_TIMEOUT: u128 = 60 * 60 * 24 * 7;

pub const DVM_CONFIGRC_FILENAME: &str = ".dvmrc";
pub const DVM_CONFIGRC_KEY_DENO_VERSION: &str = "deno_version";
pub const DVM_CONFIGRC_KEY_REGISTRY_VERSION: &str = "registry_version";
pub const DVM_CONFIGRC_KEY_REGISTRY_BINARY: &str = "registry_binary";

pub const DVM_VERSION_CANARY: &str = "canary";
pub const DVM_VERSION_LATEST: &str = "latest";
pub const DVM_VERSION_SYSTEM: &str = "system";
pub const DVM_VERSION_INVALID: &str = "N/A";

cfg_if::cfg_if! {
  if #[cfg(windows)] {
    pub const DENO_EXE: &str = "deno.exe";
  } else {
    pub const DENO_EXE: &str = "deno";
  }
}

pub const AFTER_HELP: &str = "\x1b[33mEXAMPLE:\x1b[39m
  dvm install 1.3.2     Install v1.3.2 release
  dvm install           Install the latest available version
  dvm use 1.0.0         Use v1.0.0 release
  dvm use latest        Use the latest alias that comes with dvm, equivalent to *
  dvm use canary        Use the canary version of the Deno

\x1b[33mNOTE:\x1b[39m
  To remove, delete, or uninstall dvm - just remove the \x1b[36m`$DVM_DIR`\x1b[39m folder (usually \x1b[36m`~/.dvm`\x1b[39m)";

pub const COMPLETIONS_HELP: &str = "Output shell completion script to standard output.
  \x1b[35m
  dvm completions bash > /usr/local/etc/bash_completion.d/dvm.bash
  source /usr/local/etc/bash_completion.d/dvm.bash\x1b[39m";
