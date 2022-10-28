#!/bin/bash

# Build shared library
cargo build --release || exit $?

# Install built library
plugins=${ROFI_PLUGINS_DIR}
pluginsdir=$(pkgconf --variable pluginsdir rofi)
pluginsdir=${pluginsdir:-"/usr/lib/rofi/"}
libname="librofi_vscode_mode.so"

install_cmd="install ./target/release/$libname $pluginsdir"

echo "Installing \"$libname\" in \"$pluginsdir\""
[ -w $pluginsdir ] && $install_cmd || sudo $install_cmd
