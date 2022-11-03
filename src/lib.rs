//! A very handy Rofi menu to open recent Visual Studio Code workspacess and files
//!
//! [rofi::VSCodeRecentMode] provides a [Rofi](https://github.com/davatorium/rofi) mode named `vscode-recent` to open recent items in VSCode.
//!
//! This plugin can be configured with environment variables:
//! - `ROFI_VSCODE_FLAVOR=[code|code-insiders|code-oss|vscodium]` sets the preferred VSCode flavor to be used
//! - `ROFI_VSCODE_ICON_MODE=[none|theme|nerd]` controls how icons are displayed
//! - `ROFI_VSCODE_ICON_FONT` controls the font to render the icon glyphs in case the `nerd` option is chosen
//! - `ROFI_VSCODE_ICON_COLOR` controls the color of the font in case the `nerd` option is chosen
//!
//! For more details please see the README in the repository.

pub mod vscode;

// Expose modules

pub mod utils;

#[cfg(feature = "rofi")]
pub mod rofi;

#[cfg(feature = "rofi")]
pub use rofi_mode;

// Export modes

#[cfg(feature = "rofi")]
rofi_mode::export_mode!(rofi::VSCodeRecentMode);
