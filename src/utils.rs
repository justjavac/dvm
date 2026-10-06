use crate::configrc::rc_get_with_fix;
use crate::consts::{DENO_EXE, DVM_CACHE_PATH_PREFIX, DVM_CANARY_PATH_PREFIX, DVM_CONFIGRC_KEY_DENO_VERSION};
use crate::version::VersionArg;
use anyhow::Result;
use colored::Colorize;
use dirs::home_dir;
use semver::{Version, VersionReq};
use std::env;
use std::fs::write;
use std::io::{stdin, stdout, BufReader, Read, Write};
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::time;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::NamedTempFile;

/// Print an error to stderr in red, in the `error: <message>` style used
/// throughout dvm's CLI. Centralized here so the prefix stays consistent and
/// single-colon.
pub fn print_error(err: &dyn std::fmt::Display) {
  eprintln!("{} {}", "error:".red().bold(), err);
}

/// Atomically write `content` to `path` by writing to a temp file in the same
/// directory and then renaming it into place.  This guarantees the destination
/// file is never left in a half-written state if the process crashes mid-write.
pub fn atomic_write<P: AsRef<Path>>(path: P, content: &[u8]) -> std::io::Result<()> {
  let path = path.as_ref();
  let dir = path
    .parent()
    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "path has no parent"))?;

  // Ensure the target directory exists
  std::fs::create_dir_all(dir)?;

  // Create a temp file in the same directory so rename is atomic
  let mut tmp = NamedTempFile::new_in(dir)?;
  tmp.write_all(content)?;
  tmp.flush()?;

  // Atomically replace the destination
  tmp.persist(path)?;

  Ok(())
}

pub fn run_with_spinner(
  message: String,
  finish_message: String,
  f: impl FnOnce() -> Result<()>,
) -> Result<()> {
  let spinner = indicatif::ProgressBar::new_spinner().with_message(message);
  spinner.set_style(
    indicatif::ProgressStyle::default_spinner()
      .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
      .template("{spinner:.green} {msg}")
      .unwrap(),
  );
  spinner.enable_steady_tick(time::Duration::from_millis(100));
  let result = f();
  match &result {
    Ok(()) => {
      spinner.finish_with_message(format!("{} in {:.2}s", finish_message, spinner.elapsed().as_secs_f32()));
    }
    Err(_) => {
      spinner.finish_and_clear();
    }
  }
  result
}

pub fn prompt_request(prompt: &str) -> bool {
  print!("{} (Y/n)", prompt);

  let _ = stdout().flush();
  let mut buffer = [0; 1];
  let confirm = BufReader::new(stdin())
    .read(&mut buffer)
    .ok()
    .map(|_| buffer[0] as char)
    .unwrap_or_else(|| 'y');
  confirm == '\n' || confirm == '\r' || confirm.eq_ignore_ascii_case(&'y')
}

pub fn check_is_deactivated() -> bool {
  let mut home = dvm_root();
  home.push(".deactivated");
  home.exists() && home.is_file()
}

pub fn now() -> u128 {
  SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|it| it.as_millis())
    .unwrap_or(0)
}

pub fn update_stub(version: &str) -> std::io::Result<()> {
  let mut home = dvm_versions();
  home.push(version);
  if home.is_dir() {
    home.push(".dvmstub");
    write(home, now().to_string())?;
  }
  Ok(())
}

pub fn is_exact_version(input: &str) -> bool {
  Version::parse(input).is_ok()
}

pub fn best_version<'a, T>(choices: T, required: VersionReq) -> Option<Version>
where
  T: IntoIterator<Item = &'a str>,
{
  choices
    .into_iter()
    .filter_map(|v| {
      let version = Version::parse(v).ok()?;
      required.matches(&version).then_some(version)
    })
    .max_by(|a, b| a.partial_cmp(b).unwrap())
}

///
/// Find and load the dvmrc
/// local -> user -> default
pub fn load_dvmrc() -> VersionArg {
  rc_get_with_fix(DVM_CONFIGRC_KEY_DENO_VERSION)
    .ok()
    .and_then(|v| VersionArg::from_str(&v).ok())
    .unwrap_or_else(|| VersionArg::Range(VersionReq::parse("*").expect("\"*\" is a valid VersionReq")))
}

pub fn dvm_root() -> PathBuf {
  env::var_os("DVM_DIR").map(PathBuf::from).unwrap_or_else(|| {
    // Note: on Windows, the $HOME environment variable may be set by users or by
    // third party software, but it is non-standard and should not be relied upon.
    home_dir()
      .map(|it| it.join(".dvm"))
      .unwrap_or_else(|| std::env::temp_dir().join(".dvm"))
  })
}

pub fn dvm_versions() -> PathBuf {
  let mut home = dvm_root();
  home.push(DVM_CACHE_PATH_PREFIX);
  home
}

pub fn deno_canary_path() -> PathBuf {
  let dvm_dir = dvm_root().join(DVM_CANARY_PATH_PREFIX);
  dvm_dir.join(DENO_EXE)
}

/// Path to the file that stores the current canary build hash.
/// Used to skip re-downloading when the latest canary is already installed.
pub fn canary_hash_path() -> PathBuf {
  dvm_root().join(DVM_CANARY_PATH_PREFIX).join(".canary-hash")
}

/// CGQAQ: Put hardlink to executable to this file,
///        and prepend this folder to env when dvm activated.
pub fn deno_bin_path() -> PathBuf {
  let dvm_bin_dir = dvm_root().join("bin");
  dvm_bin_dir.join(DENO_EXE)
}

/// dvm's bin directory, i.e. the directory that must come first on `PATH`
/// for the version selected by `dvm use` to win.
pub fn dvm_bin_dir() -> PathBuf {
  deno_bin_path()
    .parent()
    .map(Path::to_path_buf)
    .unwrap_or_else(|| dvm_root().join("bin"))
}

/// Where the `deno` command found on `PATH` actually points, relative to
/// dvm's bin directory.
#[derive(Debug, PartialEq, Eq)]
pub enum DenoResolution {
  /// `deno` resolves into dvm's bin directory — version switching works.
  Dvm,
  /// `deno` resolves to another installation that shadows dvm's hard link,
  /// e.g. a deno installed by winget/scoop that sits earlier on `PATH`.
  Shadowed(PathBuf),
  /// No `deno` on `PATH` at all.
  NotOnPath,
}

/// Resolve `deno` on the current `PATH` and classify it against dvm's bin
/// directory. See <https://github.com/justjavac/dvm/issues/244>.
pub fn deno_resolution() -> DenoResolution {
  classify_deno_resolution(which::which("deno").ok().as_deref(), &dvm_bin_dir())
}

/// Check whether dvm's bin directory is already present in the PATH env var.
/// Uses proper path-component comparison (not substring matching) and is
/// case-insensitive on Windows.
pub fn dvm_bin_on_path() -> bool {
  let bin_dir = dvm_bin_dir();
  std::env::var_os("PATH")
    .iter()
    .flat_map(|p| std::env::split_paths(p))
    .any(|entry| paths_equal(&entry, &bin_dir))
}

/// Compare two paths for equality, case-insensitively on Windows.
#[cfg(not(windows))]
fn paths_equal(a: &Path, b: &Path) -> bool {
  a == b
}

#[cfg(windows)]
fn paths_equal(a: &Path, b: &Path) -> bool {
  fn normalize(p: &Path) -> String {
    p.to_string_lossy().replace('/', "\\").to_lowercase()
  }
  normalize(a) == normalize(b)
}

fn classify_deno_resolution(resolved: Option<&Path>, bin_dir: &Path) -> DenoResolution {
  match resolved {
    None => DenoResolution::NotOnPath,
    Some(path) if path_contains_dir(path, bin_dir) => DenoResolution::Dvm,
    Some(path) => DenoResolution::Shadowed(path.to_path_buf()),
  }
}

/// Is `path` inside directory `dir`? Component-aware, so `/a/bin2/x` does not
/// count as inside `/a/bin`. Case-insensitive on Windows.
#[cfg(not(windows))]
fn path_contains_dir(path: &Path, dir: &Path) -> bool {
  path.starts_with(dir)
}

/// Is `path` inside directory `dir`? Component-aware, so `/a/bin2/x` does not
/// count as inside `/a/bin`. Case-insensitive on Windows.
#[cfg(windows)]
fn path_contains_dir(path: &Path, dir: &Path) -> bool {
  fn normalize(p: &Path) -> String {
    p.to_string_lossy().replace('/', "\\").to_lowercase()
  }
  let mut dir = normalize(dir);
  if !dir.ends_with('\\') {
    dir.push('\\');
  }
  normalize(path).starts_with(&dir)
}

/// Remove dvm's `deno` hard link so the system-wide deno takes over again.
/// A missing link is not an error: `dvm use system` and `dvm deactivate` are
/// both expected to be idempotent, and neither has a link to remove before the
/// first `dvm use`.
pub fn remove_deno_bin_link() -> std::io::Result<()> {
  match std::fs::remove_file(deno_bin_path()) {
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
    result => result,
  }
}

/// Create a link at `deno_bin_path()` pointing to `src`.
///
/// Tries, in order: hard link → symlink → copy.  Hard links are the most
/// efficient (zero-copy) but fail across filesystems.  Symlinks also fail on
/// some Windows configurations (non-admin users).  Copying always works but
/// uses extra disk space.
pub fn link_deno_bin(src: &Path) -> Result<()> {
  let dst = deno_bin_path();

  if let Some(parent) = dst.parent() {
    std::fs::create_dir_all(parent)?;
  }

  // 1. Hard link — fall through to symlink on failure (e.g. cross-filesystem)
  if std::fs::hard_link(src, &dst).is_ok() {
    return Ok(());
  }

  // 2. Symlink — fall through to copy on failure (e.g. Windows without admin)
  #[cfg(unix)]
  {
    if std::os::unix::fs::symlink(src, &dst).is_ok() {
      return Ok(());
    }
  }
  #[cfg(windows)]
  {
    if std::os::windows::fs::symlink_file(src, &dst).is_ok() {
      return Ok(());
    }
  }

  // 3. Copy (last resort)
  std::fs::copy(src, &dst)?;
  Ok(())
}

pub fn deno_version_path(version: &Version) -> PathBuf {
  let dvm_dir = dvm_root().join(format!("{}/{}", DVM_CACHE_PATH_PREFIX, version));
  dvm_dir.join(DENO_EXE)
}

#[inline]
pub fn is_semver(version: &str) -> bool {
  Version::parse(version).is_ok()
}

#[inline]
pub fn is_http_like_url(url: &str) -> bool {
  url.starts_with("http://") || url.starts_with("https://")
}

#[cfg(test)]
mod tests {
  use super::*;
  use semver::VersionReq;

  #[test]
  fn test_best_version() {
    let versions = [
      "0.8.5",
      "0.8.0",
      "0.9.0",
      "1.0.0",
      "1.0.0-alpha",
      "1.0.0-beta",
      "0.5.0",
      "2.0.0",
    ];
    assert_eq!(
      best_version(versions.iter().map(AsRef::as_ref), VersionReq::parse("*").unwrap()),
      Some(Version::parse("2.0.0").unwrap())
    );
    assert_eq!(
      best_version(versions.iter().map(AsRef::as_ref), VersionReq::parse("^1").unwrap()),
      Some(Version::parse("1.0.0").unwrap())
    );
    assert_eq!(
      best_version(versions.iter().map(AsRef::as_ref), VersionReq::parse("~0.8").unwrap()),
      Some(Version::parse("0.8.5").unwrap())
    );
  }

  #[test]
  fn deno_resolution_classification() {
    let bin_dir = Path::new("/home/u/.dvm/bin");

    assert_eq!(classify_deno_resolution(None, bin_dir), DenoResolution::NotOnPath);
    assert_eq!(
      classify_deno_resolution(Some(Path::new("/home/u/.dvm/bin/deno")), bin_dir),
      DenoResolution::Dvm
    );
    assert_eq!(
      classify_deno_resolution(Some(Path::new("/usr/local/bin/deno")), bin_dir),
      DenoResolution::Shadowed(PathBuf::from("/usr/local/bin/deno"))
    );
  }

  #[test]
  fn path_contains_dir_is_component_aware() {
    let bin_dir = Path::new("/home/u/.dvm/bin");

    assert!(path_contains_dir(Path::new("/home/u/.dvm/bin/deno"), bin_dir));
    // `bin2` merely shares a string prefix with `bin` — must not match.
    assert!(!path_contains_dir(Path::new("/home/u/.dvm/bin2/deno"), bin_dir));
    // A path shorter than the bin dir cannot be inside it.
    assert!(!path_contains_dir(Path::new("/home/u/.dvm"), bin_dir));
  }

  #[cfg(windows)]
  #[test]
  fn path_contains_dir_is_case_insensitive_on_windows() {
    let bin_dir = Path::new("C:\\Users\\me\\.dvm\\bin");

    assert!(path_contains_dir(
      Path::new("c:\\users\\me\\.dvm\\bin\\deno.exe"),
      bin_dir
    ));
    assert!(path_contains_dir(Path::new("C:/Users/me/.dvm/bin/deno.exe"), bin_dir));
    assert!(!path_contains_dir(
      Path::new("C:\\Users\\me\\.dvm\\bin2\\deno.exe"),
      bin_dir
    ));
  }
}
