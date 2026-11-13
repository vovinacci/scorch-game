# Scorch: `make` alone lists the targets. Targets are in alphabetical order; each has its .PHONY line beside it.

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

# Python tools (rumdl, prek, ruff, shfmt, shellcheck, fonttools) live in .venv, installed from the hash-pinned
# requirements.txt.
VENV = .venv
BIN = $(VENV)/bin

.DEFAULT_GOAL := help

.PHONY: ascii-check
ascii-check: ## Check that code and config are ASCII (see tools/check_ascii.sh)
	tools/check_ascii.sh $$(find src tools build.rs Cargo.toml .cargo Makefile shell.nix flake.nix mise.toml \
		requirements.txt requirements-build.txt prek.toml ruff.toml .rumdl.toml .editorconfig -type f)

.PHONY: balance
balance: ## Print the balance tables: weapons, firing order, digging out, deflector, batteries, guidance, bot types, difficulty, ladder
	$(call CARGO,test --release balance_ -- --ignored --nocapture --test-threads=1)

.PHONY: build
build: ## Build optimised, without running
	$(call CARGO,build --release)

.PHONY: check
check: ## Type-check, fast
	$(call CARGO,check)

.PHONY: clean
clean: ## Remove build output and the Python tools
	cargo clean
	rm -rf $(VENV)

.PHONY: docs
docs: tools ## Rewrap and fix Markdown docs (120 columns)
	$(BIN)/rumdl fmt .

.PHONY: docs-check
docs-check: tools ## Check Markdown docs
	$(BIN)/rumdl check .

.PHONY: fmt
fmt: docs python shell ## Format everything: Rust, Markdown, Python and shell
	cargo fmt

.PHONY: fmt-check
fmt-check: ## Check Rust formatting
	cargo fmt --check

.PHONY: help
help: ## List the targets
	@grep -hE '^[a-z-]+:.*## ' $(MAKEFILE_LIST) | awk 'BEGIN { FS = ":.*## " } { printf "  %-14s %s\n", $$1, $$2 }'

.PHONY: hooks
hooks: tools ## Run all git hooks on every file
	$(BIN)/prek run --all-files

.PHONY: hooks-install
hooks-install: tools ## Install the git hooks (needs a git repository)
	$(BIN)/prek install --hook-type pre-commit --hook-type commit-msg --hook-type pre-push

.PHONY: lint
lint: ## Clippy on code and tests, warnings are errors
	$(call CARGO,clippy --all-targets -- -D warnings)

.PHONY: python
python: tools ## Fix and format Python scripts (ruff)
	$(BIN)/ruff check --fix tools
	$(BIN)/ruff format tools

.PHONY: python-check
python-check: tools ## Check Python scripts (ruff)
	$(BIN)/ruff check tools
	$(BIN)/ruff format --check tools

.PHONY: release
release: ## Build optimised and play
	$(call CARGO,run --release)

.PHONY: run
run: ## Build in debug mode and play
	$(call CARGO,run)

.PHONY: shell
shell: tools ## Format shell scripts (shfmt)
	$(BIN)/shfmt -i 2 -ci -w tools

.PHONY: shell-check
shell-check: tools ## Check shell scripts (shfmt, shellcheck)
	$(BIN)/shfmt -i 2 -ci -d tools
	$(BIN)/shellcheck tools/*.sh

.PHONY: test
test: ## Run the tests
	$(call CARGO,test)

.PHONY: tools
tools: $(VENV)/.installed ## Install the Python tools into .venv

$(VENV)/.installed: requirements.txt
	$(call NEW_VENV,$(VENV))
	$(BIN)/pip install --quiet --disable-pip-version-check --require-hashes -r requirements.txt
	touch $@
