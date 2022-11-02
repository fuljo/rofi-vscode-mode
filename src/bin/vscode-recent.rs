//! Print paths of recent Visual Studio Code workspacess and files
//!
//! This plugin can be configured with environment variables:
//! - `ROFI_VSCODE_DIST=[code|code-oss|vscodium]` sets the preferred VSCode distribution to be used
//!
//! For more details please see the README in the repository.

use clap::Parser;
use rofi_vscode_mode::{
    utils::determine_vscode_distribution,
    vscode::{tildify, workspaces::recently_opened_from_storage, Distribution},
};

/// Print paths of recent Visual Studio Code workspaces and files
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Visual Studio Code distribution (code, codeoss, vscodium)
    #[arg(short, long)]
    dist: Option<Distribution>,

    /// Show full paths, without contracting the home directory
    #[arg(short = 'F', long, default_value_t = false)]
    full_paths: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let distribution = match args.dist {
        Some(dist) => dist,                       // use provided
        None => determine_vscode_distribution()?, // fallback to ENV variable or detect
    };

    let entries = recently_opened_from_storage(&distribution)?;
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
