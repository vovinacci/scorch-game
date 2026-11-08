# On NixOS, cargo runs inside nix-shell so the game finds its libraries (GL, X11, ALSA).
# Elsewhere, cargo runs directly.
ifneq ($(wildcard /etc/NIXOS),)
  CARGO = nix-shell --run "cargo $(1)"
  # The Python from shell.nix (3.15), whatever the shell around make has.
  NEW_VENV = nix-shell --run "python3 -m venv --clear $(1)"
else
  CARGO = cargo $(1)
  NEW_VENV = python3 -m venv --clear $(1)
endif

# Python tools (rumdl, prek, ruff, shfmt, shellcheck, fonttools) live in .venv, installed from the hash-pinned requirements.txt.
VENV = .venv
BIN = $(VENV)/bin

.PHONY: run release build check test lint fmt fmt-check docs docs-check shell shell-check python python-check ascii-check tools hooks hooks-install clean

run:            ## Build in debug mode and play
	$(call CARGO,run)

release:        ## Build optimized and play
	$(call CARGO,run --release)

build:
	$(call CARGO,build --release)

check:          ## Type-check, fast
	$(call CARGO,check)

test:
	$(call CARGO,test)

lint:
	$(call CARGO,clippy --all-targets -- -D warnings)

fmt: docs shell python ## Format Rust code, Markdown docs, shell and Python scripts
	cargo fmt

fmt-check:
	cargo fmt --check

docs: tools     ## Rewrap and fix Markdown docs (120 columns)
	$(BIN)/rumdl fmt .

docs-check: tools
	$(BIN)/rumdl check .

shell: tools    ## Format shell scripts (shfmt)
	$(BIN)/shfmt -i 2 -ci -w tools

shell-check: tools
	$(BIN)/shfmt -i 2 -ci -d tools
	$(BIN)/shellcheck tools/*.sh

python: tools   ## Fix and format Python scripts (ruff)
	$(BIN)/ruff check --fix tools
	$(BIN)/ruff format tools

python-check: tools
	$(BIN)/ruff check tools
	$(BIN)/ruff format --check tools

ascii-check:    ## Code and config must be ASCII (see tools/check_ascii.sh)
	tools/check_ascii.sh $$(find src tools build.rs Cargo.toml .cargo Makefile shell.nix flake.nix mise.toml requirements.txt prek.toml .rumdl.toml .editorconfig -type f)

tools: $(VENV)/.installed  ## Install the Python tools into .venv

$(VENV)/.installed: requirements.txt
	$(call NEW_VENV,$(VENV))
	$(BIN)/pip install --quiet --disable-pip-version-check --require-hashes -r requirements.txt
	touch $@

hooks-install: tools  ## Install the git hooks (needs a git repository)
	$(BIN)/prek install --hook-type pre-commit --hook-type commit-msg --hook-type pre-push

hooks: tools    ## Run all git hooks on every file
	$(BIN)/prek run --all-files

clean:
	cargo clean
	rm -rf $(VENV)
