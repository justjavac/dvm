//! Regression tests for <https://github.com/justjavac/dvm/issues/243>.
//!
//! `https://dvm.deno.dev` was a Deno Deploy Classic deployment that stopped
//! serving on 2026-07-20, and `https://deno.land/x/dvm` only serves stale
//! tagged snapshots. The documented install commands must therefore fetch
//! `install.sh` / `install.ps1` from raw.githubusercontent.com, where they
//! always track the `main` branch.

use std::path::PathBuf;

const SH_URL: &str = "https://raw.githubusercontent.com/justjavac/dvm/main/install.sh";
const PS_URL: &str = "https://raw.githubusercontent.com/justjavac/dvm/main/install.ps1";

const READMES: [&str; 2] = ["README.md", "README_zh-cn.md"];

fn repo_root() -> PathBuf {
  PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_readme(name: &str) -> String {
  std::fs::read_to_string(repo_root().join(name)).unwrap_or_else(|e| panic!("failed to read {name}: {e}"))
}

#[test]
fn readmes_use_raw_github_install_urls() {
  for name in READMES {
    let content = read_readme(name);
    assert!(content.contains(SH_URL), "{name} must install via {SH_URL}");
    assert!(content.contains(PS_URL), "{name} must install via {PS_URL}");
  }
}

#[test]
fn readmes_do_not_reference_dead_install_hosts() {
  for name in READMES {
    let content = read_readme(name);
    assert!(
      !content.contains("dvm.deno.dev"),
      "{name} must not reference the sunset Deno Deploy Classic host"
    );
    assert!(
      !content.contains("deno.land/x/dvm"),
      "{name} must not reference the stale deno.land/x snapshot"
    );
  }
}

#[test]
fn install_scripts_exist_at_repo_root() {
  // The raw URLs above resolve to these files on the `main` branch.
  for name in ["install.sh", "install.ps1"] {
    let path = repo_root().join(name);
    assert!(path.is_file(), "{} must exist", path.display());
  }
}
