//! Common utilities

const ENV_FLAVOR: &str = "ROFI_VSCODE_FLAVOR";

use super::vscode::Flavor;
use anyhow::anyhow;
use std::{env, str::FromStr};

/// Determine the VSCode flavor
///
/// First it looks up the environment variable `ROFI_VSCODE_FLAVOR`.
/// If it is not set, it tries to auto-detect the flavor
///
/// # Errors
/// The function fails if the env. variable contains an unrecognized value,
/// or if the variable is not set and a suitable flavor cannot be detected.
pub fn determine_vscode_flavor() -> anyhow::Result<Flavor> {
    if let Ok(val) = env::var(ENV_FLAVOR) {
        Flavor::from_str(&val)
    } else {
        Flavor::detect()
            .ok_or_else(|| anyhow!("Could not find any suitable VSCode flavor"))
            .copied()
    }
}
