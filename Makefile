# On NixOS, cargo runs inside nix-shell so the game finds its GL/X11/Wayland libraries.
# Elsewhere, cargo runs directly.
# The same goes for prek and rumdl, which shell.nix provides.
ifneq ($(wildcard /etc/NIXOS),)
  CARGO = nix-shell --run "cargo $(1)"
  TOOL = nix-shell --run "$(1)"
else
  CARGO = cargo $(1)
  TOOL = $(1)
endif

.PHONY: run release build check test lint fmt fmt-check docs docs-check ascii-check hooks hooks-install clean

run:            ## Build in debug mode and play
	$(call CARGO,run)

release:        ## Build optimized and play
	$(call CARGO,run --release)

build:
	$(call CARGO,build --release)

check:          ## Type-check, fast
	cargo check

test:
	cargo test

lint:
	cargo clippy -- -D warnings

fmt:            ## Format Rust code and Markdown docs
	cargo fmt
	$(call TOOL,rumdl fmt .)

fmt-check:
	cargo fmt --check

docs:           ## Rewrap and fix Markdown docs (120 columns)
	$(call TOOL,rumdl fmt .)

docs-check:
	$(call TOOL,rumdl check .)

ascii-check:    ## Code and config must be ASCII (see tools/check_ascii.sh)
	tools/check_ascii.sh $$(find src tools build.rs Cargo.toml Makefile shell.nix mise.toml prek.toml .rumdl.toml .editorconfig -type f)

hooks-install:  ## Install the git hooks (needs a git repository)
	$(call TOOL,prek install --hook-type pre-commit --hook-type commit-msg --hook-type pre-push)

hooks:          ## Run all git hooks on every file
	$(call TOOL,prek run --all-files)

clean:
	cargo clean
