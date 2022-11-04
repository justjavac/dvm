pub const REGISTRY_OFFICIAL: &str = "https://dl.deno.land/";
#[allow(unused)]
pub const REGISTRY_CN: &str = "https://dl.deno.js.cn/";
pub const REGISTRY_LATEST_RELEASE_PATH: &str = "release-latest.txt";
pub const REGISTRY_LATEST_CANARY_PATH: &str = "canary-latest.txt";

pub const DVM_CACHE_PATH_PREFIX: &str = "versions";
pub const DVM_CANARY_PATH_PREFIX: &str = "canary";
pub const DVM_CACHE_INVALID_TIMEOUT: u128 = 60 * 60 * 24 * 7;

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
