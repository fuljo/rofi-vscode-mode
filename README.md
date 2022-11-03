# VSCode mode for Rofi

[![CI Workflow](https://github.com/fuljo/rofi-vscode-mode/actions/workflows/ci.yml/badge.svg)](https://github.com/fuljo/rofi-vscode-mode/actions)


A very handy Rofi menu to open recent Visual Studio Code workspacess and files, written in Rust.

![Demonstration of open menu](assets/demo_papirus_icons.png)

Main features:
- A custom-implemented Rofi mode (a.k.a. plugin) named `vscode-recent`, to open recent workspaces and files.
- The `vscode-recent` command line tool to print paths of recent workspaces and files to stdout. Pair it with a selection tool like [dmenu](https://tools.suckless.org/dmenu/), [fzf](https://github.com/junegunn/fzf) or similar.
- Entries are taken from VSCode's _File->Open Recent_ menu.
- Delete entries from recently opened (also affects VSCode).
- Support for [remote](https://code.visualstudio.com/docs/remote/remote-overview) workspaces, files and folders.
- Support for different flavors: [Visual Studio Code](https://code.visualstudio.com), [Visual Studio Code Insiders](https://code.visualstudio.com/insiders), [Code - OSS](https://github.com/microsoft/vscode) and [VSCodium](https://vscodium.com).

This project was largely inspired by [rofi-code](https://github.com/Coffelius).
Many thanks to [@Coffelius](https://github.com/Coffelius) for writing it, and to [@SabrinaJewson](https://github.com/SabrinaJewson) for providing Rust bindings to Rofi's C plugin interface.

If you are curious, I wrote a short [wiki article](https://github.com/fuljo/rofi-vscode-mode/wiki/How-it-works) explaining how this tool gets the recent items and opens them.

## Install

### Build from source
You can choose to build and install only the `vscode-recent` tool, only the plugin or both.

First clone this repository
```sh
git clone https://github.com/fuljo/rofi-vscode-mode
```

Then get [Rust](https://www.rust-lang.org/tools/install) as you prefer.

Then install the needed dependencies
```sh
# Ubuntu / Debian
apt-get install \
  build-essential pkg-config libsqlite3-dev \
  rofi-dev libpango1.0-dev  # only needed for the rofi plugin

# Arch
pacman -S \
  make pkg-config sqlite \
  rofi # only needed for the rofi plugin
```

Then run `make` according to your choice:
```sh
# Binary and plugin
make install

# Binary only
make install.bin

# Plugin only
make install.plugin
```

## Usage

### As a Rofi mode
This library introduces a new mode named `vscode-recent`.
You can run it standalone with the command
```sh
rofi -show vscode-recent -modi vscode-recent
```
or add it to your default _modi_ in `~/.config/rofi/config.rasi`, like so
```
configuration {
	modi: "drun,run,window,vscode-recent";
    show-icons:                 true;
	drun-display-format:        "{name}";
	window-format:              "{w} | {c} | {t}";
}
```

I highly reccommend assigning a keyboard shortcut for this; for example I use <kbd>Mod</kbd> + <kbd>C</kbd> to run `rofi -show vscode-recent` (after adding it to my default modi).

When an item is selected, press:
- <kbd>Enter</kbd> to open it
- <kbd>Shift</kbd>+<kbd>Del</kbd> to permanently delete it from the list

:warning: Item deletion works by updating the recent items list in VSCode's state database. Do it at your own risk. Please use this feature when VSCode is closed, otherwise your changes may be overwritten.

### As a command line tool
If you prefer something other than Rofi to select your entry, we also provide the `vscode-recent` command that simply writes out the paths line by line. You can then pair it with your favourite selection tool, like [dmenu](https://tools.suckless.org/dmenu/) or [fzf](https://github.com/junegunn/fz).

You can use the `-c` option to set the preferred flavor and the `-F` option to set the desired ouput format:
- `label` (default) will show the "tildified" path, which needs to be expanded. Remote entries are not shown.
  ```sh
  sh -c "code $(vscode-recent | dmenu)"
  ```
- `absolute-path` will show the full path. Remote entries are not shown.
  ```sh
  code $(vscode-recent | dmenu)
  ```
- `uri` will show the locl or remote URI, read [this](https://code.visualstudio.com/docs/remote/troubleshooting#_ssh-tips) for hints on how to open it. Remote entries are shown.


## Configuration
Various aspects of this plugin can be configured with environment variables.
If you are using keyboard shortcuts to launch Rofi, make sure that these variables are set in Usagethe shell that launches Rofi, e.g. by adding an `export` statement to your `~/.bash_profile`.

Configuration of the theme and everything else is left to Rofi itself.

### VSCode flavor
Multiple VSCode flavors exist for Linux, see the [Arch Wiki](https://wiki.archlinux.org/title/Visual_Studio_Code) for details.

By default this plugin will try to detect a flavor for which both a command in `$PATH` and a configuration directory exist.
If you want to select it by hand, set `ROFI_VSCODE_FLAVOR` with one of the following values (case insensitive):

| `ROFI_VSCODE_FLAVOR` | Flavor                      | Command         | Configuration directory      |
| -------------------- | --------------------------- | --------------- | ---------------------------- |
| `code`               | Visual Studio Code          | `code`          | `~/.config/Code/`            |
| `code-insiders`      | Visual Studio Code Insiders | `code-insiders` | `~/.config/Code - Insiders/` |
| `code-oss`           | Code - OSS                  | `code-oss`      | `~/.config/Code - OSS/`      |
| `vscodium`           | VSCodium                    | `codium`        | `~/.config/VSCodium/`        |

### Icons
By default icons from Rofi's current icon theme are shown besides the entries. You have three choices:
- Set `ROFI_VSCODE_ICON_MODE=none` to disable icons
- Set `ROFI_VSCODE_ICON_MODE=theme` to use the icons from Rofi's current icon theme
- Set `ROFI_VSCODE_ICON_MODE=nerd` to use icons from a [Nerd Font](https://www.nerdfonts.com/).<br>
  The font can be chosen by setting `ROFI_VSCODE_ICON_FONT=fontname` (defaults to monospace) and its color by setting
  `ROFI_VSCODE_ICON_COLOR` to an `#rrggbb` or `#rrggbbaa` value.

A different icon is shown for workspaces, files and folders.

<img src="assets/demo_no_icons.png" width="49%"> <img src="assets/demo_nerd_icons.png" width="49%">

## Contributing

If you like this little piece of software and would like to improve it, please fork the repo and create a pull request. Your contributions are greatly appreciated.

If you want to report a problem, please open an Issue.
Make sure you include your Rofi version and any error messages that are printed by running the mode from a terminal as described [before](#usage).

## License

This software is released under the MIT license.
