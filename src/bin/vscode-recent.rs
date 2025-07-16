//! Print paths of recent Visual Studio Code workspacess and files
//!
//! This plugin can be configured with environment variables:
//! - `ROFI_VSCODE_FLAVOR=[code|code-insiders|code-oss|vscodium]` sets the preferred VSCode flavor to be used
//!
//! For more details please see the README in the repository.

use clap::{Parser, ValueEnum};
use rofi_vscode_mode::{
    utils::determine_vscode_flavor,
    vscode::{
        workspaces::{recently_opened_from_storage, Recent},
        Flavor,
    },
};

/// How each item should be shown
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    /// Label (if provided), otherise tildified path
    ///
    /// Shows only local items
    Label,
    /// Absolute path
    ///
    /// Shows only local items
    AbsolutePath,
    /// URI
    ///
    /// Shows all items
    Uri,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Label
    }
}

/// Print paths of recent Visual Studio Code workspaces and files
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Visual Studio Code flavor (code, code-insiders, code-oss, vscodium)
    #[arg(short = 'c', long)]
    flavor: Option<Flavor>,

    /// Output format
    #[arg(short = 'F', long, value_enum, default_value_t = OutputFormat::default())]
    output_format: OutputFormat,
}

fn format_entry(entry: &Recent, output_format: &OutputFormat) -> anyhow::Result<String> {
    match output_format {
        OutputFormat::Label => entry.label().map(|s| s.to_string()),
        OutputFormat::AbsolutePath => entry.file_path().map(|p| p.to_string_lossy().to_string()),
        OutputFormat::Uri => Ok(entry.url().to_string()),
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Determine the flavor
    let flavor = match args.flavor {
        Some(flavor) => flavor,             // use provided
        None => determine_vscode_flavor()?, // fallback to ENV variable or detect
    };

    // Include non-local items? Only if we are able to open them from command line with a URI
    let local_only = match args.output_format {
        OutputFormat::Uri => false,
        OutputFormat::Label | OutputFormat::AbsolutePath => true,
    };

    // Query and print the entries
    let entries = recently_opened_from_storage(&flavor, local_only)?;
    for entry in entries {
        if let Ok(s) = format_entry(&entry, &args.output_format) {
            println!("{s}")
        }
    }
    Ok(())
}
