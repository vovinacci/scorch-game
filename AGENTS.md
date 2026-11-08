# Scorch

Scorched Earth style artillery game in Rust + macroquad.

- `docs/GUIDE.md` (and `docs/GUIDE.uk.md`): the player's guide. Plain player-facing language only: no code, build steps
  or internals.
- `docs/RELEASING.md`: GitHub, CI, releases and self-update; commits follow Conventional Commits.
- `docs/DEVELOPMENT.md`: building, dev flags, exact numbers behind the rules, translations, save files, versioning, code
  map.

## Rules for every change

- Any change players can notice (gameplay, a control, a menu, a setting) must update `docs/GUIDE.md` and
  `docs/GUIDE.uk.md` in the same change. Exact numbers and internals go in `docs/DEVELOPMENT.md`, updated likewise.
- Code and config are ASCII only (`make ascii-check`, also a commit hook). Docs and `.ftl` files are UTF-8. A line that
  truly needs other characters ends with `ascii-ok`.
- All on-screen text goes through `t!("key")` with the key in both `assets/i18n/en-GB.ftl` (British spelling) and
  `assets/i18n/uk.ftl`. Never hard-code on-screen strings; tests check the keys.
- Run `make lint` and `make test` before calling a change done. The `guide_lists_*` tests check that both guides name
  every weapon and setting value in their language.
- Check layouts in both languages with `--screen <name> --shot file.png` (see `docs/DEVELOPMENT.md`). All layout uses
  the 1280x720 virtual screen in `ui.rs`; the window may be any size (Sway tiles it).
- Markdown docs wrap at 120 columns: run `make docs` after editing them (`make docs-check` checks). Rust stays at
  rustfmt's 100. Hooks (`prek.toml`) run the same checks on commit.
- Commits and PR titles are Conventional Commits (`feat(bots): ...`); the changelog is generated from them. No trailers
  in commits or PRs (no Co-authored-by, no "Generated with").
- New saved fields need `#[serde(default)]` so older save files still load.

## Shell scripts

Shell scripts (`tools/*.sh`) follow the [Google Shell Style Guide](https://google.github.io/styleguide/shellguide.html)
with the changes below; where they disagree, this section wins. `make shell` formats with `shfmt -i 2 -ci`, and the
`shell` hook runs shfmt and shellcheck on every commit. A new script starts with:

```bash
#!/usr/bin/env bash
#
# What the script does, and why when it is not obvious.

set -euf -o pipefail
export LC_ALL="C.UTF-8"
PS4='+ $(date -u +%Y-%m-%dT%H:%M:%S%z) ${BASH_SOURCE[0]##*/}:${LINENO}: '
```

and ends with `main "$@"`, an empty line and `# EOF` (a truncated file then shows as a missing marker).

- `#!/usr/bin/env bash`, not `#!/bin/bash`: macOS ships bash 3.2 there, and NixOS has no `/bin/bash`.
- `set -euf -o pipefail`: `-u` stops on an unset variable, `-f` turns globbing off so a path is used literally. Match
  patterns with `find` (`readarray -t files < <(find "${dir}" -maxdepth 1 -name "${pattern}")`), not `set +f`. Leave
  `-e` out only in a script that must go on after a failed command, with a comment saying why.
- `LC_ALL="C.UTF-8"`: sorting and character classes behave the same on a runner and a laptop.
- `PS4` in single quotes, so every `set -x` line shows its own time, file and line.
- Functions: `function kebab-name() {`. The work goes in `main`. A script with several actions dispatches with
  `case "${1:-}"` in `main`, and its `*)` branch prints the usage to stderr and exits 1 (a bare `"$1"` would stop on
  "unbound variable" before the usage).
- Brace every expansion: `"${var}"`, not `"$var"`. Constants are `readonly`; `export` only what a child process needs.
- A required value gets a message: `"${1:?usage: ...}"` or `: "${VAR:?VAR must be set}"`.
- Errors go to stderr (`echo "..." >&2`).
- A continued pipeline ends its line with `|` (shfmt's layout). Lines up to 120 columns (`.editorconfig`).
- A script that guards something (like `check_ascii.sh` or the glibc check in `build-linux.sh`) is finished only once it
  has been seen failing on the input it exists to catch.

## Gotchas

- Test runs: use `XDG_DATA_HOME=<scratch dir>` so the user's saves stay untouched. The user may be playing at the same
  time: stop only your own process by PID, never `pkill scorch`.
- Screenshots: use `--screen <name> --shot file.png`. Grabbing the window with `grim`/`swaymsg` is unreliable (other
  outputs, blank frames).
- Never store GPU objects (Font, Texture) in a value that is dropped at exit: they get freed after the GL context is
  gone and the game segfaults. Leak them on purpose, like `FONT` in `ui.rs`.
- The font (Scorch Mono) has no emoji. Before using a new symbol, check the font has it:
  `fc-query assets/fonts/ScorchMono.ttf`.
- Ukrainian runs ~25% longer than English: give bounded text `ui::text_fit` / `ui::text_wrapped`, then check both
  languages with `--shot`.
- Fluent: messages referenced from other messages don't receive the caller's arguments, so inline plural selectors.
  Isolation marks are off (the font has no glyphs for them).
- Player names are saved in English; show them with `Player::label()`. Ukrainian sentences avoid declining names ("в
  танк Альфа"); use the `you` / `target_you` arguments for grammar.
- Text drawn over the battlefield goes inside `ui::with_shadow` (dark outline).
