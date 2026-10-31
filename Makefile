# On NixOS, cargo runs inside nix-shell so the game finds its GL/X11/Wayland libraries.
# Elsewhere, cargo runs directly.
ifneq ($(wildcard /etc/NIXOS),)
  CARGO = nix-shell --run "cargo $(1)"
else
  CARGO = cargo $(1)
endif

# Python tools (rumdl, prek, fonttools) live in .venv, installed from the hash-pinned requirements.txt.
VENV = .venv
BIN = $(VENV)/bin

.PHONY: run release build check test lint fmt fmt-check docs docs-check ascii-check tools hooks hooks-install clean

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

fmt: docs       ## Format Rust code and Markdown docs
	cargo fmt

fmt-check:
	cargo fmt --check

docs: tools     ## Rewrap and fix Markdown docs (120 columns)
	$(BIN)/rumdl fmt .

docs-check: tools
	$(BIN)/rumdl check .

ascii-check:    ## Code and config must be ASCII (see tools/check_ascii.sh)
	tools/check_ascii.sh $$(find src tools build.rs Cargo.toml .cargo Makefile shell.nix flake.nix mise.toml requirements.txt prek.toml .rumdl.toml .editorconfig -type f)

tools: $(VENV)/.installed  ## Install the Python tools into .venv

$(VENV)/.installed: requirements.txt
	python3 -m venv $(VENV)
	$(BIN)/pip install --quiet --disable-pip-version-check --require-hashes -r requirements.txt
	touch $@

hooks-install: tools  ## Install the git hooks (needs a git repository)
	$(BIN)/prek install --hook-type pre-commit --hook-type commit-msg --hook-type pre-push

hooks: tools    ## Run all git hooks on every file
	$(BIN)/prek run --all-files

clean:
	cargo clean
	rm -rf $(VENV)
