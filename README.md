# VSCode mode for Rofi

<!-- TODO: badges -->

A very handy Rofi menu to open recent Visual Studio Code workspaces.
Supports [Visual Studio Code](https://code.visualstudio.com), [Code - OSS](https://github.com/microsoft/vscode) and [VSCodium](https://vscodium.com).

This is a sort-of-clone of [rofi-code](https://github.com/Coffelius/rofi-code), but implemented in Rust as an ad-hoc _Rofi mode_ rather than using Rofi's drun interface.

Many thanks to [@Coffelius](https://github.com/Coffelius) for inspiring this project and to [@SabrinaJewson](https://github.com/SabrinaJewson) for providing Rust bindings for Rofi's plugin interface.

## Build and install

First make sure to install [Rofi](https://github.com/davatorium/rofi) 1.7 and a [Rust](https://www.rust-lang.org/tools/install) toolchain.
Then run
```sh
./install.sh
```
which will build the project and copy the produced shared library to Rofi's plugins directory.

## Usage
This library will introduce a new mode named `vscode-workspace`.
You can run it standalone
```sh
rofi -show vscode-workspace -modi vscode-workspace
```
or add it to your default _modi_ in `~/.config/rofi/config.rasi`, like so
```
configuration {
	modi: "drun,run,window,vscode-workspace";
    show-icons:                 true;
	drun-display-format:        "{name}";
	window-format:              "{w} | {c} | {t}";
}
```

I highly reccommend assigning a keyboard shortcut for this; for example I use <kbd>Mod</kbd> + <kbd>C</kbd> to run `rofi -show vscode-workspace` (after adding it to my default modi).

## Modes

### `vscode-workspace`
Shows workspaces found in `~/.config/{Code, Code - OSS, VSCodium}/User/wokrspaceStorage/` sorted by most recently used.

<!-- TODO: Add screenshot -->

## Roadmap

- [x] Support for single-folder workspaces
- [ ] Support for multi-folder workspaces
- [ ] Enable icons with an environment variable `ROFI_VSCODE_ICONS`
- [ ] Mode `vscode-recent` with items from the _File->Open Recent_ menu

## Contributing

If you like this little piece of software and would like to improve it, please fork the repo and create a pull request. Your contributions are greatly appreciated.

## License

This software is released under the MIT license.
