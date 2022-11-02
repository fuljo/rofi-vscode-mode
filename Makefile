PKGNAME ?= rofi-vscode-mode
LIBNAME ?= librofi_vscode_mode.so
BINNAME ?= vscode-recent

CARGO ?= cargo
CARGO_TARGET_DIR ?= target
CARGO_RELEASE_DIR ?= $(CARGO_TARGET_DIR)/release

# Set DESTDIR for staged installs

prefix ?= /usr
bindir ?= $(prefix)/bin
lbdir ?= $(prefix)/lib
datarootdir ?= $(prefix)/share
docdir ?= $(datarootdir)/doc
licensesdir ?= $(datarootdir)/licenses

# Find the directory to install plugins (only expand if needed)
pluginsdir_pc = $(shell pkg-config --variable pluginsdir rofi)
pluginsdir ?= $(if $(pluginsdir_pc),$(pluginsdir_pc),$(lbdir)/rofi)

# Build everything
all:
	cargo build --release

# Build only library (plugin for Rofi)
$(CARGO_RELEASE_DIR)/$(LIBNAME):
	$(CARGO) build --release --lib --features rofi

plugin: $(CARGO_RELEASE_DIR)/$(LIBNAME)

# Build only the binary
$(CARGO_RELEASE_DIR)/$(BINNAME):
	$(CARGO) build --release --bin $(BINNAME) --no-default-features

bin: $(CARGO_RELEASE_DIR)/$(BINNAME)

# Install everything
install: all
	test -w $(DESTDIR)$(pluginsdir) \
		&& install $(CARGO_RELEASE_DIR)/$(LIBNAME) $(DESTDIR)$(pluginsdir) \
		|| sudo install $(CARGO_RELEASE_DIR)/$(LIBNAME) $(DESTDIR)$(pluginsdir)

# Just install the plugin
install.plugin: $(CARGO_RELEASE_DIR)/$(LIBNAME)
	test -w $(DESTDIR)$(pluginsdir) \
		&& install $(CARGO_RELEASE_DIR)/$(LIBNAME) $(DESTDIR)$(pluginsdir) \
		|| sudo install $(CARGO_RELEASE_DIR)/$(LIBNAME) $(DESTDIR)$(pluginsdir)

# Just install the binary
install.bin: $(CARGO_RELEASE_DIR)/$(BINNAME)
	test -w $(DESTDIR)$(bindir) \
		&& install $(CARGO_RELEASE_DIR)/$(BINNAME) $(DESTDIR)$(bindir) \
		|| sudo install $(CARGO_RELEASE_DIR)/$(BINNAME) $(DESTDIR)$(bindir)

install.doc:
	test -w $(DESTDIR)$(docdir) \
		&& install README.md $(DESTDIR)$(docdir) \
		|| sudo install README.md $(DESTDIR)$(docdir)

install.licenses:
	test -w $(DESTDIR)$(licensesdir) \
		&& install LICENSE $(DESTDIR)$(licensesdir) \
		|| sudo install LICENSE $(DESTDIR)$(licensesdir)

.PHONY: all install install.plugin install.bin install.doc
