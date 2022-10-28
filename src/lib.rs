pub mod vscode;

use std::{cmp::Reverse, ffi::OsStr, fs, ops::Deref, process::Command};

use anyhow::{anyhow, Context};
use rofi_mode::{self as rofi, Action, Api, Event, Matcher};
use vscode::{workspaces_from_storage, Distribution, Workspace};

fn open_path<S, T>(cmd: S, path: T) -> anyhow::Result<Action>
where
    S: AsRef<OsStr>,
    T: AsRef<OsStr>,
{
    Command::new(cmd)
        .arg(path)
        .output()
        .with_context(|| "Could not execute VSCode")
        .map(|_| Action::Exit)
}

pub struct VSCodeWorkspaceMode<'rofi> {
    _api: Api<'rofi>,
    workspaces: Vec<Workspace>,
}

impl<'rofi> rofi_mode::Mode<'rofi> for VSCodeWorkspaceMode<'rofi> {
    const NAME: &'static str = "vscode-workspace\0";

    /// Initialization
    fn init(mut api: Api<'rofi>) -> Result<Self, ()> {
        // Set name
        api.set_display_name("Open Workspace");
        // Initialize the workspaces
        let mut workspaces = workspaces_from_storage().map_err(|e| eprint!("{}", e))?;
        // Most recent first
        workspaces.sort_by_key(|ws| Reverse(ws.mod_time));
        Ok(Self {
            _api: api,
            workspaces,
        })
    }

    /// Get the number of entries offered by the mode
    fn entries(&mut self) -> usize {
        self.workspaces.len()
    }

    fn entry_content(&self, line: usize) -> rofi::String {
        self.workspaces[line].label[..].into()
    }

    fn entry_icon(&mut self, _line: usize, _height: u32) -> Option<rofi::cairo::Surface> {
        None
    }

    fn react(&mut self, event: Event, input: &mut rofi::String) -> Action {
        let res: anyhow::Result<Action> = match event {
            Event::Cancel { selected: _ } => Ok(Action::Exit),

            Event::Ok { alt: _, selected } => {
                let workspace = &self.workspaces[selected];
                open_path(workspace.distribution.cmd(), &workspace.path)
            }

            Event::CustomInput {
                alt: _,
                selected: _,
            } => {
                let path = shellexpand::tilde(input);
                Distribution::detect_cmd()
                    .with_context(|| "Cannot find VSCode executable")
                    .and_then(|exec| open_path(&exec, path.deref()))
            }

            Event::Complete { selected } => {
                match selected {
                    Some(line) => {
                        *input = rofi::String::from(&self.workspaces[line].label);
                    }
                    None => {
                        *input = rofi::String::from("");
                    }
                }
                Ok(Action::Exit)
            }

            Event::DeleteEntry { selected } => {
                // Delete the workspace storage
                let workspace = &self.workspaces[selected];
                fs::remove_dir_all(&workspace.storage_path)
                    .with_context(|| "Could not delete workspace storage directory")
                    .map(|_| Action::Reload)
            }

            Event::CustomCommand {
                number: _,
                selected: _,
            } => Err(anyhow!("Command not supported")),
        };
        match res {
            Ok(a) => a,
            Err(e) => {
                eprint!("{:?}", e);
                Action::Exit
            }
        }
    }

    /// Check if the given matcher matches an entry
    fn matches(&self, line: usize, matcher: Matcher<'_>) -> bool {
        matcher.matches(&self.workspaces[line].label)
    }
}

rofi_mode::export_mode!(VSCodeWorkspaceMode);

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
