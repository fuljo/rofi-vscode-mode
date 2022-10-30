use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{anyhow, Context, Result};
use url::Url;
use which::which;

/// One of the possible VSCode distributions
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Distribution {
    Code,
    CodeOSS,
    VSCodium,
}

impl Distribution {
    //// The command to run the distribution
    pub fn cmd(&self) -> &str {
        match self {
            Self::Code => "code",
            Self::CodeOSS => "code-oss", // also provides `code`
            Self::VSCodium => "codium",  // also provides `vscodium`
        }
    }

    /// Path to the configuration directory of the distribution, if it exists
    pub fn config_dir(&self) -> Option<PathBuf> {
        let subdir = match self {
            Self::Code => "Code",
            Self::CodeOSS => "Code - OSS",
            Self::VSCodium => "VSCodium",
        };
        dirs::config_dir()
            .map(|mut p| {
                p.push(subdir);
                p
            })
            .filter(|p| p.exists())
    }

    /// Tries to detect the preferred distribution
    ///
    /// It returns the first distribution for which it can find both:
    /// - an executable in `$PATH`
    /// - a configuration directory
    pub fn detect() -> Option<&'static Self> {
        let candidates = &[Self::VSCodium, Self::CodeOSS, Self::Code];
        candidates
            .iter()
            .find(|d| which(d.cmd()).ok().and(d.config_dir()).is_some())
    }
}

impl FromStr for Distribution {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "code" => Ok(Self::Code),
            "codeoss" => Ok(Self::CodeOSS),
            "vscodium" | "codium" => Ok(Self::VSCodium),
            _ => Err(anyhow!("\"{}\" does not match any VSCode distribution", s)),
        }
    }
}

/// VSCode workspace and history management
///
/// For reference see VSCode's source code:
/// - [Workspaces History Main Service](https://github.com/microsoft/vscode/blob/main/src/vs/platform/workspaces/electron-main/workspacesHistoryMainService.ts)
/// - [workspaces common definitions](https://github.com/microsoft/vscode/blob/main/src/vs/platform/workspaces/common/workspaces.ts)
pub mod workspaces {
    use super::{path_from_url, tildify, Distribution};
    use std::{
        borrow::Cow,
        fmt::{self, Display},
        path::{Path, PathBuf},
    };

    use anyhow::{anyhow, Context};
    use rusqlite::Connection;
    use serde::Deserialize;
    use serde_json::Value;

    const VSCDB_HISTORY_KEY: &str = "history.recentlyOpenedPathsList";

    /// Identifies a multi-root Workspace
    ///
    /// The workspace has an associated `<name>.code-workspace` config file, which represented in the [`Self::config_path`].
    ///
    /// See [this documentation article](https://code.visualstudio.com/docs/editor/workspaces) for reference.
    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct WorkspaceIdentifier {
        /// Unique identifier of the workspace
        ///
        /// It is usually the SHA-256 of [`Self::config_path`] and identifies the storage folder for workspace data.<br>
        /// `$CODE_CONFIG_DIR/User/workspaceStorage/{id}/`
        #[allow(dead_code)]
        id: String,
        /// Location of the `.code-workspace` file
        config_path: String,
    }

    /// A recently opened item
    #[derive(Deserialize, Debug)]
    #[serde(untagged)]
    pub enum Recent {
        /// A multi-root workspace
        ///
        /// It has an associated `.code-workspace` config file
        #[serde(rename_all = "camelCase")]
        Workspace {
            workspace: WorkspaceIdentifier,
            label: Option<String>,
            remote_authority: Option<String>,
        },
        /// A workspace with a single folder
        #[serde(rename_all = "camelCase")]
        Folder {
            folder_uri: String,
            label: Option<String>,
            remote_authority: Option<String>,
        },
        /// A single file
        #[serde(rename_all = "camelCase")]
        File {
            file_uri: String,
            label: Option<String>,
            remote_authority: Option<String>,
        },
    }

    impl Recent {
        /// Returns the file path that can be used to open this recent item
        ///
        /// The function should return a path `P` such that running
        /// ```sh
        /// code $P
        /// ```
        /// in a shell will open the recent item.
        ///
        /// The path is derived from the internal `file://` URL.
        ///
        /// # Errors
        /// The call will fail if the URL has a scheme other than `file://` or if the URL path is not a valid system path.
        pub fn path(&self) -> anyhow::Result<PathBuf> {
            match self {
                Recent::Workspace {
                    workspace,
                    label: _,
                    remote_authority: _,
                } => path_from_url(&workspace.config_path),
                Recent::Folder {
                    folder_uri,
                    label: _,
                    remote_authority: _,
                } => path_from_url(folder_uri),
                Recent::File {
                    file_uri,
                    label: _,
                    remote_authority: _,
                } => path_from_url(file_uri),
            }
        }

        /// Returns a displayable label
        ///
        /// If the `label` field is assigned it will be returned as-is.
        /// Otherwise, the label will be computed by turning the `file://`
        /// URL to a path and replacing the `$HOME` prefix with `~`.
        ///
        /// # Errors
        /// The call will fail if the URL has a scheme other than `file://` or if the URL path is not a valid system path.
        pub fn label(&self) -> anyhow::Result<Cow<str>> {
            match self {
                Recent::Workspace {
                    workspace: _,
                    label,
                    remote_authority: _,
                } => {
                    // Use given label or forge it
                    label
                        .as_ref()
                        .map(Cow::from)
                        .ok_or(())
                        .or_else(|_| Ok(Cow::from(tildify(&self.path()?))))
                }
                Recent::Folder {
                    folder_uri: _,
                    label,
                    remote_authority: _,
                } => {
                    // Use given label or forge it
                    label
                        .as_ref()
                        .map(Cow::from)
                        .ok_or(())
                        .or_else(|_| Ok(Cow::from(tildify(&self.path()?))))
                }
                Recent::File {
                    file_uri: _,
                    label,
                    remote_authority: _,
                } => {
                    // Use given label or forge it
                    label
                        .as_ref()
                        .map(Cow::from)
                        .ok_or(())
                        .or_else(|_| Ok(Cow::from(tildify(&self.path()?))))
                }
            }
        }
    }

    impl Display for Recent {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let label = self.label().map_err(|_| fmt::Error)?;
            write!(f, "{}", label)
        }
    }

    /// Get recently opened workspaces, files and folders for specific distribution
    ///
    /// # Warning
    /// Workspaces that fail to deserialize to known data structures will be ignored.
    /// Workspaces that have invalid URIs are still deserialized.
    /// However, since this data is written by VSCode itself afther extensive checking,
    /// it is unlikely that there are any invalid URIs.
    ///
    /// The entries will be looked up from VSCode's global storage inside the given `config_dir` configuration directory
    fn get_history_entries(config_dir: &Path) -> anyhow::Result<Vec<Recent>> {
        // Reference from `restoreRecentlyOpened` in
        // https://github.com/microsoft/vscode/blob/main/src/vs/platform/workspaces/common/workspaces.ts

        // Open the DB
        let db_path = config_dir
            .join("User")
            .join("globalStorage")
            .join("state.vscdb");

        let conn = Connection::open(&db_path)
            .with_context(|| format!("Could not open database {:?}", &db_path))?;

        // Retrieve the JSON value of the property
        let res: Value = conn
            .query_row(
                "SELECT value FROM ItemTable WHERE key = (?)",
                [VSCDB_HISTORY_KEY],
                |r| r.get(0),
            )
            .with_context(|| {
                format!(
                    "Could not retrieve key \"{}\" from \"{:?}\" as JSON",
                    VSCDB_HISTORY_KEY, &db_path
                )
            })?;

        // Deserialize the JSON array to our datatypes
        let entries = res["entries"]
            .as_array()
            .ok_or_else(|| anyhow!("History object's \"entries\" attribute is not an array"))?;

        let entries = entries
            .iter()
            .filter_map(|e| -> Option<Recent> { serde_json::from_value(e.to_owned()).ok() })
            .collect();

        Ok(entries)
    }

    /// Get recently opened workspaces, files and folders
    ///
    /// This function will retrieve the items from the _global storage_ of the
    /// given `distribution`. The items are sorted from the most to the least recent
    ///
    /// # Warning
    /// Workspaces that fail to deserialize to known data structures will be ignored.
    /// Workspaces that have invalid URIs are still deserialized.
    /// However, since this data is written by VSCode itself afther extensive checking,
    /// it is unlikely that there are any invalid URIs.
    ///
    /// The entries will be looked up from VSCode's global storage
    pub fn recently_opened_from_storage(
        distribution: &Distribution,
    ) -> anyhow::Result<Vec<Recent>> {
        let config_dir = distribution.config_dir().ok_or_else(|| {
            anyhow!(
                "Could not find configuration directory for \"{:?}\"",
                distribution
            )
        })?;
        get_history_entries(&config_dir)
    }
}

/// Converts an URL to its corresponding file path
///
/// # Errors
/// The conversion will fail if the provided `input` is not a valid URL, or if it doesn't have the `file://` scheme.
fn path_from_url(input: &str) -> Result<PathBuf> {
    let url = Url::parse(input).with_context(|| format!("Cannot parse URL {}", input))?;
    match url.scheme() {
        "file" => url
            .to_file_path()
            .map_err(|_| anyhow!("Invalid URL file path {}", url)),
        _ => Err(anyhow!("Unsupported URL scheme {}", url.scheme())),
    }
}

/// Replace the home directory prefix of `path` with `~`
///
/// If the prefix is not present or the home directory cannot be determined,
/// the path is returned as is.
pub fn tildify(path: &Path) -> String {
    dirs::home_dir()
        .and_then(|home| path.strip_prefix(home).ok())
        .map(|suff| {
            let mut path = PathBuf::new();
            path.push("~");
            path.push(suff);
            Cow::from(path)
        })
        .unwrap_or_else(|| Cow::from(path))
        .to_string_lossy()
        .to_string()
}

/// Expands the `~` prefix in `path` to the user's home directory
///
/// If the prefix is not present or the home directory cannot be determined,
/// the path is returned as is.
pub fn untildify(path: &str) -> PathBuf {
    let path = PathBuf::from(&path);
    dirs::home_dir()
        .and_then(|home| {
            path.strip_prefix(PathBuf::from("~"))
                .map(|suffix| home.join(suffix))
                .ok()
        })
        .unwrap_or(path)
}
