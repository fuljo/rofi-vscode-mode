//! Common utilities

const ENV_DIST: &str = "ROFI_VSCODE_DIST";

use super::vscode::Distribution;
use anyhow::anyhow;
use std::{env, str::FromStr};

/// Determine the VSCode distribution
///
/// First it looks up the environment variable `ROFI_VSCODE_DIST`.
/// If it is not set, it tries to auto-detect the distribution
///
/// # Errors
/// The function fails if the env. variable contains an unrecognized value,
/// or if the variable is not set and a suitable distribution cannot be detected.
pub fn determine_vscode_distribution() -> anyhow::Result<Distribution> {
    if let Ok(val) = env::var(ENV_DIST) {
        Distribution::from_str(&val)
    } else {
        Distribution::detect()
            .ok_or_else(|| anyhow!("Could not find any suitable VSCode distribution"))
            .map(|d| *d)
    }
}
