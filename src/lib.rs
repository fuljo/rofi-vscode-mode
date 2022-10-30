pub mod vscode;

use std::{ffi::OsStr, process::Command, str::FromStr};

use anyhow::{anyhow, Context};
use rofi_mode::{self as rofi, Action, Api, Event, Matcher};
use vscode::{
    untildify,
    workspaces::{recently_opened_from_storage, store_recently_opened, Recent},
    Distribution,
};

const ENV_DIST: &str = "ROFI_VSCODE_DIST";

// Open recent workspaces, files and folders with VSCode
pub struct VSCodeRecentMode<'rofi> {
    /// Binding to the Rofi api
    _api: Api<'rofi>,
    /// The entries that will be displayed
    entries: Vec<Recent>,
    /// The selected VSCode distribution
    distribution: Distribution,
}

impl<'rofi> rofi_mode::Mode<'rofi> for VSCodeRecentMode<'rofi> {
    const NAME: &'static str = "vscode-recent\0";

    /// Initialization
    fn init(mut api: Api<'rofi>) -> Result<Self, ()> {
        // Set name
        api.set_display_name("Open Recent");
        // Initialize vscode distribution
        let distribution = determine_vscode_distribution().map_err(|e| eprint!("{:?}", e))?;
        // Initialize the entries
        let entries =
            recently_opened_from_storage(&distribution).map_err(|e| eprint!("{:?}", e))?;

        Ok(VSCodeRecentMode {
            _api: api,
            entries,
            distribution,
        })
    }

    /// Get the number of entries offered by the mode
    fn entries(&mut self) -> usize {
        self.entries.len()
    }

    fn entry_content(&self, line: usize) -> rofi::String {
        match self.entries[line].label() {
            Ok(label) => rofi::String::from(label.as_ref()),
            Err(e) => {
                eprint!("{}", e);
                rofi::String::new()
            }
        }
    }

    fn entry_icon(&mut self, _line: usize, _height: u32) -> Option<rofi::cairo::Surface> {
        // TODO: Implement icons
        None
    }

    fn react(&mut self, event: Event, input: &mut rofi::String) -> Action {
        let res: anyhow::Result<Action> = match event {
            // Pressed Escape key
            Event::Cancel { selected: _ } => Ok(Action::Exit),

            // Selected an item
            Event::Ok { alt: _, selected } => self.entries[selected]
                .path()
                .and_then(|p| open_path(self.distribution.cmd(), &p)),

            // Selected a custom input (not in list)
            Event::CustomInput {
                alt: _,
                selected: _,
            } => {
                let path = untildify(input);
                open_path(self.distribution.cmd(), &path)
            }

            // Autocomplete input from selected entry
            Event::Complete { selected } => {
                if let Some(line) = selected {
                    if let Ok(label) = self.entries[line].label() {
                        *input = rofi::String::from(label.as_ref());
                    }
                }
                Ok(Action::Reset)
            }

            // Delete selected entry
            Event::DeleteEntry { selected } => {
                self.entries.remove(selected);
                store_recently_opened(&self.distribution, &self.entries).map(|_| Action::Reload)
            }

            // User ran a custom command
            Event::CustomCommand {
                number: _,
                selected: _,
            } => Err(anyhow!("Command not supported")),
        };
        // Handle errors
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
        match self.entries[line].label() {
            Ok(label) => matcher.matches(&label),
            Err(_) => false,
        }
    }
}

rofi_mode::export_mode!(VSCodeRecentMode);

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

fn determine_vscode_distribution() -> anyhow::Result<Distribution> {
    if let Ok(val) = std::env::var(ENV_DIST) {
        Distribution::from_str(&val)
    } else {
        Distribution::detect()
            .ok_or_else(|| anyhow!("Could not find any suitable VSCode distribution"))
            .map(|d| *d)
    }
}
