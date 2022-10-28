use std::fs::{self, File};
use std::io::BufReader;

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{anyhow, Context, Result};
use serde_json::Value;
use strum::{EnumIter, IntoEnumIterator};
use url::Url;
use which::which;

/// Represents one of the possible VSCode distributions
#[derive(PartialEq, Clone, Copy, Debug, EnumIter)]
pub enum Distribution {
    Code,
    CodeOSS,
    VSCodium,
}

impl Distribution {
    //// The command to run the distribution
    pub fn cmd(&self) -> &str {
        match self {
            Distribution::Code => "code",
            Distribution::CodeOSS => "code-oss", // also provides `code`
            Distribution::VSCodium => "vscodium", // also provides `codium`
        }
    }

    /// Path to the executable to run the distribution, if it exists
    pub fn exec(&self) -> Option<PathBuf> {
        which(self.cmd()).ok()
    }

    /// Path to the configuration directory of the distribution, if it exists
    pub fn config_dir(&self) -> Option<PathBuf> {
        let subdir = match self {
            Distribution::Code => "Code",
            Distribution::CodeOSS => "Code - OSS",
            Distribution::VSCodium => "VSCodium",
        };
        dirs::config_dir()
            .map(|mut p| {
                p.push(subdir);
                p
            })
            .filter(|p| p.exists())
    }

    /// Automatically detect the command to run VSCode and return a path to the executable
    pub fn detect_cmd() -> Option<PathBuf> {
        Self::iter().find_map(|d| d.exec())
    }
}

#[derive(PartialEq, Debug)]
pub struct Workspace {
    /// The label that will be shown to the user
    pub label: String,
    /// The distribution this belongs to
    pub distribution: Distribution,
    /// The path of workspace storage
    pub storage_path: PathBuf,
    /// The path of the workspace directory
    pub path: PathBuf,
    /// The last modification timestamp
    pub mod_time: SystemTime,
}

impl Workspace {
    pub fn from_workspace_storage(
        distribution: &Distribution,
        storage_path: &Path,
    ) -> Result<Workspace> {
        // Read path from workspace.json
        let ws_json_path = storage_path.join("workspace.json");
        let ws_json_file = File::open(&ws_json_path)
            .with_context(|| format!("Failed to open {:?}", ws_json_path))?;
        let ws_json: Value = serde_json::from_reader(BufReader::new(ws_json_file))
            .with_context(|| format!("Failed to read JSON from {:?}", ws_json_path))?;
        let ws_url: &str = ws_json["folder"].as_str().with_context(|| {
            format!(
                "Cannot read `folder` property of {:?} as string",
                ws_json_path
            )
        })?;
        let ws_url = Url::parse(ws_url).with_context(|| format!("Cannot parse URL {}", ws_url))?;
        let ws_path = match ws_url.scheme() {
            "file" => match ws_url.to_file_path() {
                Ok(p) => p,
                Err(_) => return Err(anyhow!("Invalid URL file path {}", ws_url)),
            },
            _ => return Err(anyhow!("Unsupported URL scheme {}", ws_url.scheme())),
        };

        // Create a user-friendly label
        let ws_label = Self::label_from_path(&ws_path, true);

        // Read last modified timestamp from state.vscdb
        let ws_state_path = storage_path.join("state.vscdb");
        let ws_mod_time = fs::metadata(&ws_state_path)
            .and_then(|md| md.modified())
            .with_context(|| format!("Could not load metadata for {:?}", ws_state_path))?;

        // Build workspace
        Ok(Workspace {
            label: ws_label,
            distribution: *distribution,
            storage_path: storage_path.into(),
            path: ws_path,
            mod_time: ws_mod_time,
        })
    }

    fn label_from_path(ws_path: &Path, contract_home: bool) -> String {
        let contracted_path = match contract_home {
            true => dirs::home_dir()
                .and_then(|home| ws_path.strip_prefix(home).ok())
                .map(|suff| {
                    let mut path = PathBuf::new();
                    path.push("~");
                    path.push(suff);
                    path
                }),
            false => None,
        };
        match contracted_path {
            Some(p) => p.to_string_lossy().to_string(),
            None => ws_path.to_string_lossy().to_string(),
        }
    }
}

pub fn workspaces_from_storage() -> Result<Vec<Workspace>> {
    let mut workspaces = vec![];

    for dist in Distribution::iter() {
        if let Some(config_dir) = dist.config_dir() {
            let path = config_dir.join("User").join("workspaceStorage");
            if let Ok(rd) = fs::read_dir(path) {
                for entry in rd.flatten() {
                    let res = Workspace::from_workspace_storage(&dist, &entry.path())
                        .with_context(|| "Failed to load workspace");
                    match res {
                        Ok(ws) => workspaces.push(ws),
                        Err(e) => eprint!("{:?}", e),
                    };
                }
            }
        }
    }

    workspaces.sort_by_key(|ws| ws.mod_time);

    Ok(workspaces)
}
