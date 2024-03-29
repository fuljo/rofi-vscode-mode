PKGNAME := rofi-vscode-mode
LIBNAME := librofi_vscode_mode.so
BINNAME := vscode-recent

CARGO ?= cargo
CARGO_TARGET_DIR ?= target
CARGO_RELEASE_DIR ?= $(CARGO_TARGET_DIR)/release

# Set DESTDIR for staged installs

prefix ?= /usr/local
bindir ?= $(prefix)/bin
libdir ?= $(prefix)/lib
datarootdir ?= $(prefix)/share
docdir ?= $(datarootdir)/doc/$(PKGNAME)
licensesdir ?= $(datarootdir)/licenses/$(PKGNAME)

# Find the directory to install plugins (only expand if needed)
pluginsdir_pc = $(shell pkg-config --variable pluginsdir rofi)
pluginsdir ?= $(if $(pluginsdir_pc),$(pluginsdir_pc),$(libdir)/rofi)

# Build everything
all:
	cargo build --release

# Build only library (plugin for Rofi)
plugin:
	$(CARGO) build --release --lib --features rofi

# Build only the binary
bin:
	$(CARGO) build --release --bin $(BINNAME) --no-default-features

# Install everything
install: install.plugin install.bin

# Just install the plugin
install.plugin:
	install -Dt $(DESTDIR)$(pluginsdir) $(CARGO_RELEASE_DIR)/$(LIBNAME)

# Just install the binary
install.bin:
	install -Dt $(DESTDIR)$(bindir) $(CARGO_RELEASE_DIR)/$(BINNAME)

install.doc:
	install -Dt $(DESTDIR)$(docdir) README.md

install.licenses:
	install -Dt $(DESTDIR)$(licensesdir) LICENSE

clean:
	$(CARGO) clean

.PHONY: all plugin bin install install.plugin install.bin install.doc install.licences clean
