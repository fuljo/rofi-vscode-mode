//! Print paths of recent Visual Studio Code workspacess and files
//!
//! This plugin can be configured with environment variables:
//! - `ROFI_VSCODE_FLAVOR=[code|code-insiders|code-oss|vscodium]` sets the preferred VSCode flavor to be used
//!
//! For more details please see the README in the repository.

use clap::Parser;
use rofi_vscode_mode::{
    utils::determine_vscode_flavor,
    vscode::{tildify, workspaces::recently_opened_from_storage, Flavor},
};

/// Print paths of recent Visual Studio Code workspaces and files
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Visual Studio Code flavor (code, code-insiders, code-oss, vscodium)
    #[arg(short, long)]
    flavor: Option<Flavor>,

    /// Show full paths, without contracting the home directory
    #[arg(short = 'F', long, default_value_t = false)]
    full_paths: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let flavor = match args.flavor {
        Some(flavor) => flavor,             // use provided
        None => determine_vscode_flavor()?, // fallback to ENV variable or detect
    };

    let entries = recently_opened_from_storage(&flavor)?;
    for entry in entries {
        let s = entry.path().map(|p| match args.full_paths {
            true => p.to_string_lossy().to_string(),
            false => tildify(&p),
        });
        if let Ok(s) = s {
            println!("{}", s)
        }
    }
    Ok(())
}
