//! Visual Studio Code utilities

use std::{
    borrow::Cow,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
    str::FromStr,
};

use anyhow::{anyhow, Context, Result};
use rusqlite::{Connection, OpenFlags};
use which::which;

use self::workspaces::Recent;

#[allow(dead_code)]
const SCHEME_FILE: &str = "file";
#[allow(dead_code)]
const SCHEME_REMOTE: &str = "vscode-remote";
#[allow(dead_code)]
const SCHEME_VIRTUAL: &str = "vscode-vfs";

/// One of the possible VSCode flavors
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Flavor {
    Code,
    CodeInsiders,
    CodeOSS,
    VSCodium,
}

impl Flavor {
    //// The command to run the flavor
    pub fn cmd(&self) -> &str {
        match self {
            Self::Code => "code",
            Self::CodeInsiders => "code-insiders",
            Self::CodeOSS => "code-oss", // also provides `code`
            Self::VSCodium => "codium",  // also provides `vscodium`
        }
    }

    /// Path to the configuration directory of the flavor, if it exists
    pub fn config_dir(&self) -> Option<PathBuf> {
        let subdir = match self {
            Self::Code => "Code",
            Self::CodeInsiders => "Code - Insiders",
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

    /// Tries to detect the preferred flavor
    ///
    /// It returns the first flavor for which it can find both:
    /// - an executable in `$PATH`
    /// - a configuration directory
    pub fn detect() -> Option<&'static Self> {
        let candidates = &[
            Self::VSCodium,
            Self::CodeOSS,
            Self::CodeInsiders,
            Self::Code,
        ];
        candidates
            .iter()
            .find(|d| which(d.cmd()).ok().and(d.config_dir()).is_some())
    }

    /// Opens a recent item
    ///
    /// It will execute a command to open the given item
    ///
    /// # Errors
    /// Opening the item may fail if [self.cmd()] is not found in `PATH`.
    /// Currently, we support the `file://`, `vscode-remote://` and `vscode-vfs://` schemes.
    pub fn open_recent(&self, recent: &Recent) -> anyhow::Result<()> {
        let mut cmd = Command::new(self.cmd());

        let url = recent.url().to_string();
        match recent {
            Recent::Workspace {
                workspace: _,
                label: _,
                remote_authority: _,
            } => {
                cmd.arg("--file-uri").arg(url);
            }
            Recent::Folder {
                folder_uri: _,
                label: _,
                remote_authority: _,
            } => {
                cmd.arg("--folder-uri").arg(url);
            }
            Recent::File {
                file_uri: _,
                label: _,
                remote_authority: _,
            } => {
                cmd.arg("--file-uri").arg(url);
            }
        }
        cmd.output()
            .map(|_| ())
            .with_context(|| format!("Could not open entry with {}", self.cmd()))
    }

    /// Opens the given path
    ///
    /// # Errors
    /// Opening the item may fail if [self.cmd()] is not found in `PATH` or if the command fails for some other reason.
    pub fn open_local_path<S: AsRef<OsStr>>(&self, path: S) -> Result<()> {
        Command::new(self.cmd())
            .arg(path)
            .output()
            .map(|_| ())
            .with_context(|| "Could not execute VSCode")
    }
}

impl FromStr for Flavor {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_ref() {
            "code" => Ok(Self::Code),
            "code-insiders" => Ok(Self::CodeInsiders),
            "code-oss" => Ok(Self::CodeOSS),
            "vscodium" | "codium" => Ok(Self::VSCodium),
            _ => Err(anyhow!("\"{}\" does not match any VSCode flavor", s)),
        }
    }
}

/// VSCode workspace and history management
///
/// For reference see VSCode's source code:
/// - [Workspaces History Main Service](https://github.com/microsoft/vscode/blob/main/src/vs/platform/workspaces/electron-main/workspacesHistoryMainService.ts)
/// - [workspaces common definitions](https://github.com/microsoft/vscode/blob/main/src/vs/platform/workspaces/common/workspaces.ts)
pub mod workspaces {
    use super::{open_state_db, tildify, Flavor, SCHEME_FILE};
    use std::{
        borrow::Cow,
        fmt::{self, Display},
        path::{Path, PathBuf},
    };

    use anyhow::{anyhow, Context};
    use rusqlite::{params, OpenFlags};
    use serde::{Deserialize, Serialize};
    use serde_json::{json, Value};
    use url::Url;

    const VSCDB_HISTORY_KEY: &str = "history.recentlyOpenedPathsList";

    /// Identifies a multi-root Workspace
    ///
    /// The workspace has an associated `<name>.code-workspace` config file, which represented in the [`Self::config_path`].
    ///
    /// See [this documentation article](https://code.visualstudio.com/docs/editor/workspaces) for reference.
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    pub struct WorkspaceIdentifier {
        /// Unique identifier of the workspace
        ///
        /// It is usually the SHA-256 of [`Self::config_path`] and identifies the storage folder for workspace data.<br>
        /// `$CODE_CONFIG_DIR/User/workspaceStorage/{id}/`
        #[allow(dead_code)]
        pub id: String,
        /// Location of the `.code-workspace` file
        pub config_path: Url,
    }

    /// A recently opened item
    ///
    /// Each item has:
    /// - an [Self::url] that locates the file/folder
    /// - a [Self::label] that is shown to the user
    /// - an optional [Self::remote] that identifies the remote host of this item
    ///
    /// # Remote
    /// Any item can either be located on the local filesystem, a remote one or a virtual one.
    ///
    /// Remote filesystems are supported by the [Remote Development](https://code.visualstudio.com/docs/remote/remote-overview) feature.
    /// In this case, the [Self::remote] identifies such host and has the form `{type}+{id}`.
    ///
    /// The following remotes are supported:
    /// - [`ssh-remote+{host}`](https://code.visualstudio.com/docs/remote/ssh)
    /// - [`dev-container+{container_id}`](https://code.visualstudio.com/docs/devcontainers/containers)
    /// - [`wsl+{wsl_id}`](https://code.visualstudio.com/docs/remote/wsl)
    ///
    /// # Virtual
    /// Virtual filesystems are used in [virtual workspaces](https://code.visualstudio.com/api/extension-guides/virtual-workspaces),
    /// for example to open Github Repositories directly within VSCode.
    ///
    /// # URL
    /// There are three types of URLs:
    /// - local URLs with the form `file://{path}` that locate items on the local filesystem
    /// - remote URLs with the form `vscode-remote://{remote}/{path}` that locate items in remote hosts
    /// - virtual URLs with the form `vscode-vfs://{provider}/{path}` that locate items in virtual filesystems (e.g. GitHub)
    ///
    /// # Path
    /// The item's `path` is a filesystem path that can be used to open it:
    /// ```sh
    /// # Local item
    /// code {path}
    ///
    /// # Remote item
    /// code --remote {remote} {path}
    /// ```
    ///
    /// We currently support only local paths via [Self::file_path].
    #[derive(Serialize, Deserialize, Debug)]
    #[serde(untagged)]
    pub enum Recent {
        /// A multi-root workspace
        ///
        /// It has an associated `.code-workspace` config file
        #[serde(rename_all = "camelCase")]
        Workspace {
            workspace: WorkspaceIdentifier,
            #[serde(skip_serializing_if = "Option::is_none")]
            label: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            remote_authority: Option<String>,
        },
        /// A workspace with a single folder
        #[serde(rename_all = "camelCase")]
        Folder {
            folder_uri: Url,
            #[serde(skip_serializing_if = "Option::is_none")]
            label: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            remote_authority: Option<String>,
        },
        /// A single file
        #[serde(rename_all = "camelCase")]
        File {
            file_uri: Url,
            #[serde(skip_serializing_if = "Option::is_none")]
            label: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            remote_authority: Option<String>,
        },
    }

    impl Recent {
        /// Locates the item in a local or remote filesystem
        pub fn url(&self) -> &Url {
            match self {
                Self::Workspace {
                    workspace,
                    label: _,
                    remote_authority: _,
                } => &workspace.config_path,
                Self::Folder {
                    folder_uri,
                    label: _,
                    remote_authority: _,
                } => folder_uri,
                Self::File {
                    file_uri,
                    label: _,
                    remote_authority: _,
                } => file_uri,
            }
        }

        /// Tells whether the item is local
        pub fn is_local(&self) -> bool {
            return self.url().scheme() == SCHEME_FILE;
        }

        /// Returns the remote where this item is located, if any
        pub fn remote(&self) -> Option<&str> {
            match self {
                Self::Workspace {
                    workspace: _,
                    label: _,
                    remote_authority,
                } => remote_authority.as_deref(),
                Self::Folder {
                    folder_uri: _,
                    label: _,
                    remote_authority,
                } => remote_authority.as_deref(),
                Self::File {
                    file_uri: _,
                    label: _,
                    remote_authority,
                } => remote_authority.as_deref(),
            }
        }

        /// Returns the local file path that can be used to open this recent item
        ///
        /// # Errors
        /// The call will fail if the URL's scheme is other than `file://`, or if the URL path is not a valid system path.
        pub fn file_path(&self) -> anyhow::Result<PathBuf> {
            let url = self.url();
            match url.scheme() {
                SCHEME_FILE => url
                    .to_file_path()
                    .map_err(|_| anyhow!("Could not get path from file url {}", url)),
                _ => Err(anyhow!("Not a file url {}", url)),
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
                        .or_else(|_| Ok(Cow::from(tildify(&self.file_path()?))))
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
                        .or_else(|_| Ok(Cow::from(tildify(&self.file_path()?))))
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
                        .or_else(|_| Ok(Cow::from(tildify(&self.file_path()?))))
                }
            }
        }

        /// Name of the icon to display from the icon theme
        ///
        /// This name can be used to query the icon from the icon theme
        ///
        /// See the [Freedesktop documentation](https://specifications.freedesktop.org/icon-naming-spec/latest/ar01s04.html)
        pub fn icon_name(&self) -> &str {
            match self {
                Self::Workspace {
                    workspace: _,
                    label: _,
                    remote_authority: _,
                } => "visual-studio-code",
                Self::Folder {
                    folder_uri: _,
                    label: _,
                    remote_authority: _,
                } => "folder",
                Self::File {
                    file_uri: _,
                    label: _,
                    remote_authority: _,
                } => "text-x-generic",
            }
        }

        /// Icon glyph from nerd font
        ///
        /// See the [Nerd Fonts Cheat Sheet](https://www.nerdfonts.com/cheat-sheet)
        pub fn nerd_icon(&self) -> &str {
            match self {
                Self::Workspace {
                    workspace: _,
                    label: _,
                    remote_authority: _,
                } => "\u{fb0f}",
                Self::Folder {
                    folder_uri: _,
                    label: _,
                    remote_authority: _,
                } => "\u{f74a}",
                Self::File {
                    file_uri: _,
                    label: _,
                    remote_authority: _,
                } => "\u{f713}",
            }
        }
    }

    impl Display for Recent {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let label = self.label().map_err(|_| fmt::Error)?;
            write!(f, "{}", label)
        }
    }

    /// Get recently opened workspaces, files and folders for specific flavor
    ///
    /// If `local_only` is set, recent items for which [Recent::is_local()] does not hold will be discarded.
    /// This is useful if you need to open the items by path.
    ///
    /// # Warning
    /// Workspaces that fail to deserialize to known data structures will be ignored.
    ///
    /// The entries will be looked up from VSCode's global storage inside the given `config_dir` configuration directory
    fn get_history_entries(config_dir: &Path, local_only: bool) -> anyhow::Result<Vec<Recent>> {
        // Reference from `restoreRecentlyOpened` in
        // https://github.com/microsoft/vscode/blob/main/src/vs/platform/workspaces/common/workspaces.ts

        // Open the DB
        let open_flags = Some(OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX);
        let conn = open_state_db(config_dir, open_flags)?;

        // Retrieve the JSON value of the property
        let res: Value = conn
            .query_row(
                "SELECT value FROM ItemTable WHERE key = (?)",
                [VSCDB_HISTORY_KEY],
                |r| r.get(0),
            )
            .with_context(|| {
                format!(
                    "Could not retrieve key \"{}\" from state DB as JSON",
                    VSCDB_HISTORY_KEY
                )
            })?;

        // Deserialize the JSON array to our datatypes
        let entries = res["entries"]
            .as_array()
            .ok_or_else(|| anyhow!("History object's \"entries\" attribute is not an array"))?;

        let filter: fn(&Recent) -> bool = match local_only {
            false => |_| true,
            true => |e| e.is_local(),
        };
        let entries = entries
            .iter()
            .filter_map(|e| -> Option<Recent> { serde_json::from_value(e.to_owned()).ok() })
            .filter(filter)
            .collect();

        Ok(entries)
    }

    /// Get recently opened workspaces, files and folders
    ///
    /// This function will retrieve the items from the _global storage_ of the
    /// given `flavor`. The items are sorted from the most to the least recent
    ///
    /// If `local_only` is set, recent items for which [Recent::is_local()] does not hold will be discarded.
    /// This is useful if you need to open the items by path.
    ///
    /// # Warning
    /// Workspaces that fail to deserialize to known data structures will be ignored.
    ///
    /// The entries will be looked up from VSCode's global storage
    pub fn recently_opened_from_storage(
        flavor: &Flavor,
        local_only: bool,
    ) -> anyhow::Result<Vec<Recent>> {
        let config_dir = flavor.config_dir().ok_or_else(|| {
            anyhow!(
                "Could not find configuration directory for \"{:?}\"",
                flavor
            )
        })?;
        get_history_entries(&config_dir, local_only)
    }

    /// Store the workspaces into VSCode's state
    ///
    /// Performs the reverse operation of [recently_opened_from_storage],
    /// see its documentation for details.
    pub fn store_recently_opened(flavor: &Flavor, entries: &[Recent]) -> anyhow::Result<()> {
        let config_dir = flavor.config_dir().ok_or_else(|| {
            anyhow!(
                "Could not find configuration directory for \"{:?}\"",
                flavor
            )
        })?;

        // Open DB
        let open_flags = Some(OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX);
        let conn = open_state_db(&config_dir, open_flags)?;

        // Serialize to JSON
        let value = json!({
            "entries": entries,
        });

        // Update DB
        conn.execute(
            "UPDATE ItemTable SET value = (?2) WHERE key = (?1)",
            params![VSCDB_HISTORY_KEY, value],
        )
        .with_context(|| "Could not update state in DB")
        .map(|_| ())
    }
}

fn open_state_db(config_dir: &Path, open_flags: Option<OpenFlags>) -> anyhow::Result<Connection> {
    let open_flags = open_flags.unwrap_or_default();
    let db_path = config_dir
        .join("User")
        .join("globalStorage")
        .join("state.vscdb");

    Connection::open_with_flags(&db_path, open_flags)
        .with_context(|| format!("Could not open database {:?}", &db_path))
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
