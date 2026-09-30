//! Full Disk Access has no query API. What every implementation does instead is read a path that
//! macOS protects and see whether it is allowed — this crate does the same, and says so.

use std::fs::read_dir;
use std::path::PathBuf;

/// Paths macOS protects under Full Disk Access and that exist on a stock system.
///
/// More than one is probed because a user can delete any single app's container.
const PROTECTED: [&str; 4] = [
    "Library/Safari",
    "Library/Containers/com.apple.stocks",
    "Library/Application Support/com.apple.TCC",
    "Library/Cookies",
];

/// Whether a protected path could be read.
///
/// This is a probe, not a query: a `true` means one protected directory was readable just now.
pub(crate) fn authorized() -> bool {
    let Some(home) = home() else { return false };
    PROTECTED.iter().any(|path| read_dir(home.join(path)).is_ok())
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
