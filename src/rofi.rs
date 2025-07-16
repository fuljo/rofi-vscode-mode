//! Rofi modes and related utilities

use std::env;

use super::utils::determine_vscode_flavor;
use super::vscode::{
    untildify,
    workspaces::{recently_opened_from_storage, store_recently_opened, Recent},
    Flavor,
};
use anyhow::anyhow;
use pangocairo::{self, cairo, pango};
use rofi_mode::{self as rofi, Action, Api, Event, Matcher};

const ENV_ICON_MODE: &str = "ROFI_VSCODE_ICON_MODE";
const ENV_ICON_FONT: &str = "ROFI_VSCODE_ICON_FONT";
const ENV_ICON_COLOR: &str = "ROFI_VSCODE_ICON_COLOR";

/// How to show icons next to items
#[derive(Debug, Default)]
pub enum IconMode {
    /// No icons (default)
    None,
    /// From current icon theme
    #[default]
    Theme,
    /// From the given nerd font
    Nerd,
}

/// Configuration for the icons
#[derive(Debug)]
pub struct IconConfig {
    /// How icons are shown
    mode: IconMode,
    /// Nerd font name to render icons
    font: String,
    /// Color to render icon font
    color: RGBAColor,
}

// Open recent workspaces, files and folders with VSCode
pub struct VSCodeRecentMode<'rofi> {
    /// Binding to the Rofi api
    api: Api<'rofi>,
    /// The entries that will be displayed
    entries: Vec<Recent>,
    /// The selected VSCode flavor
    flavor: Flavor,
    /// Configuration to render icons
    icon_config: IconConfig,
}

impl<'rofi> rofi_mode::Mode<'rofi> for VSCodeRecentMode<'rofi> {
    const NAME: &'static str = "vscode-recent\0";

    /// Initialization
    fn init(mut api: Api<'rofi>) -> Result<Self, ()> {
        // Set name
        api.set_display_name("Open Recent");
        // Initialize vscode flavor
        let flavor = determine_vscode_flavor().map_err(|e| eprint!("{e:?}"))?;
        // Initialize the entries
        let entries = recently_opened_from_storage(&flavor, false).map_err(|e| eprint!("{e:?}"))?;

        let icon_config = determine_icon_config().map_err(|e| eprint!("{e:?}"))?;

        Ok(VSCodeRecentMode {
            api,
            entries,
            flavor,
            icon_config,
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
                eprint!("{e}");
                rofi::String::new()
            }
        }
    }

    fn entry_icon(&mut self, line: usize, height: u32) -> Option<cairo::Surface> {
        let entry = &self.entries[line];
        match self.icon_config.mode {
            IconMode::None => None,
            IconMode::Theme => self
                .api
                .query_icon(entry.icon_name(), height)
                .wait(&mut self.api)
                .map_err(|e| eprintln!("{e}"))
                .ok(),
            IconMode::Nerd => draw_nerd_icon(
                entry.nerd_icon(),
                &self.icon_config.font,
                self.icon_config.color,
                height,
            )
            .map_err(|e| eprintln!("{e}"))
            .ok(),
        }
    }

    fn react(&mut self, event: Event, input: &mut rofi::String) -> Action {
        let res: anyhow::Result<Action> = match event {
            // Pressed Escape key
            Event::Cancel { selected: _ } => Ok(Action::Exit),

            // Selected an item
            Event::Ok { alt: _, selected } => self
                .flavor
                .open_recent(&self.entries[selected])
                .map(|_| Action::Exit),
            // Selected a custom input (not in list)
            Event::CustomInput {
                alt: _,
                selected: _,
            } => {
                let path = untildify(input);
                self.flavor.open_local_path(path).map(|_| Action::Exit)
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
                store_recently_opened(&self.flavor, &self.entries).map(|_| Action::Reload)
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
                eprint!("{e:?}");
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

fn determine_icon_config() -> anyhow::Result<IconConfig> {
    let _mode = env::var(ENV_ICON_MODE)
        .map(|v| v.to_lowercase())
        .map(|icon_mode| match icon_mode.as_str() {
            "none" => IconMode::None,
            "theme" => IconMode::Theme,
            "nerd" => IconMode::Nerd,
            _ => IconMode::Theme,
        })
        .unwrap_or_default();

    let font = env::var(ENV_ICON_FONT).unwrap_or_else(|_| "monospace".to_string());

    let color = env::var(ENV_ICON_COLOR)
        .map_err(|_| ())
        .and_then(|s| RGBAColor::parse(&s))
        .unwrap_or_default();

    Ok(IconConfig {
        mode: _mode,
        font,
        color,
    })
}

fn draw_nerd_icon(
    text: &str,
    font: &str,
    color: RGBAColor,
    size: u32,
) -> anyhow::Result<cairo::Surface> {
    let size = i32::try_from(size)?;

    // Create drawing surface
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, size, size)?;
    let surface = unsafe { cairo::Surface::from_raw_none(surface.to_raw_none()) };
    let cr = cairo::Context::new(&surface)?;

    // Set text layout
    let layout = pangocairo::functions::create_layout(&cr);
    let font_size = f64::from(size) * 0.75;
    let desc = pango::FontDescription::from_string(&format!("{font} {font_size}"));
    layout.set_font_description(Some(&desc));
    layout.set_alignment(pango::Alignment::Center);
    layout.set_text(text);

    // Center the text
    let (ext, _) = layout.pixel_extents();
    let x = f64::from(size - ext.width()) / 2.0 - f64::from(ext.x());
    let y = f64::from(size - ext.height()) / 2.0 - f64::from(ext.y());
    cr.move_to(x, y);

    // Draw the text
    let RGBAColor(red, green, blue, alpha) = color;
    cr.set_source_rgba(red, green, blue, alpha);
    pangocairo::functions::update_layout(&cr, &layout);
    pangocairo::functions::show_layout(&cr, &layout);

    Ok(surface)
}

/// An color with Red, Green, Blue, Alpha 8-bit channels
#[derive(Debug, Copy, Clone)]
struct RGBAColor(f64, f64, f64, f64);

impl Default for RGBAColor {
    fn default() -> Self {
        RGBAColor(0.0, 0.0, 0.0, 1.0)
    }
}

impl RGBAColor {
    fn parse_channel(s: &str) -> Result<f64, ()> {
        u8::from_str_radix(s, 16)
            .map_err(|_| ())
            .map(|chan| f64::from(chan) / f64::from(u8::MAX))
    }

    /// Parse from a string of the form `#rrggbb` or `#rrggbbaa`
    fn parse(s: &str) -> Result<Self, ()> {
        match s.strip_prefix('#') {
            Some(s) => {
                match s.len() {
                    6 => {
                        // #rrggbb
                        Ok(RGBAColor(
                            Self::parse_channel(&s[0..2])?,
                            Self::parse_channel(&s[2..4])?,
                            Self::parse_channel(&s[4..6])?,
                            1.0,
                        ))
                    }
                    8 => {
                        // #rrggbbaa
                        Ok(RGBAColor(
                            Self::parse_channel(&s[0..2])?,
                            Self::parse_channel(&s[2..4])?,
                            Self::parse_channel(&s[4..6])?,
                            Self::parse_channel(&s[6..8])?,
                        ))
                    }
                    _ => Err(()),
                }
            }
            None => Err(()),
        }
    }
}
