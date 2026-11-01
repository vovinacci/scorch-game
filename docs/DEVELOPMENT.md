# Scorch: development notes

For people working on the game. The player's guide is `GUIDE.md` (and its Ukrainian copy `GUIDE.uk.md`); keep both in
sync with any change players can notice. This file holds what players don't need: how to build and test, the exact
numbers behind the rules, file formats, versioning and the code map.

## Tools

- **Rust** is pinned with [mise](https://mise.jdx.dev) in `mise.toml` (1.99.1, with rustfmt and clippy). With mise
  installed, entering this folder selects it (`mise trust` once, `mise install` if missing). CI uses the same file.
  Without mise, any Rust 1.88 or newer builds the game.
- **Python tools** (prek for git hooks, rumdl for Markdown, fonttools for the font) are pinned with hashes in
  `requirements.txt`; `make tools` installs them into `.venv`, and the other `make` targets use them from there. CI
  installs the same file, and Dependabot keeps it current.
- **NixOS:** `shell.nix` provides the graphics libraries the game loads at run time and cocogitto (commit message
  check); the `make` targets use it automatically. Elsewhere install cocogitto with `cargo install cocogitto`.

## Binaries

- **Linux:** links only glibc (and libgcc). miniquad loads `libX11.so.6`, `libXi.so.6`, `libxkbcommon.so.0` and
  `libGL.so.1` at run time (default X11 backend; Wayland desktops use XWayland), so the game uses the player's own GPU
  driver; a fully static (musl) build couldn't load it. Nothing is needed at build time, and there is no audio
  (macroquad's `audio` feature is off), so no ALSA. Release builds run on Ubuntu 22.04: glibc 2.35 or newer. On
  distributions these libraries are part of every desktop (Debian/Ubuntu: `libx11-6 libxi6 libxkbcommon0 libgl1`).
- **NixOS:** `flake.nix` packages the game and wraps it with those four libraries; `nix build` builds it (and runs the
  tests), `nix run .` plays it, players use `nix run github:vovinacci/scorch-game`. The version comes from the flake's
  commit (`SCORCH_BUILD_VERSION`, see `build.rs`). CI builds the flake; a weekly workflow updates `flake.lock`
  (Dependabot doesn't handle Nix).
- **Windows:** the C runtime is linked statically (`.cargo/config.toml`), so no Visual C++ redistributable is needed.
- **macOS:** links only system frameworks; one universal binary for Intel and Apple Silicon.

## Building and running

```text
make run       # debug build, then play
make release   # optimised build, then play
make test      # unit tests
make lint      # clippy, warnings are errors
```

On NixOS, `make` runs cargo inside `nix-shell` (see `shell.nix`). The window may be any size; all layout uses a 1280x720
virtual screen, scaled to fit.

Command line options:

| Option | What it does |
|---|---|
| `--version`, `-V` | Print the version and exit |
| `--crash-test` | Crash on purpose, to check crash reports |
| `--update` | Check for a new version and install it from the command line (same checks as the menu) |
| `--screen <name>` | Open one screen directly with a fresh game: intro, menu, about, setup, custom, settings, stats, crash, battle, pause, over (a round just won), log, shop, abandon, final. Saving is off. |
| `--shot <file.png>` | With `--screen`: save a screenshot after a moment and quit |

Self-update is off in development builds (`make run`, versions with `+`), so its question, setting and About button are
hidden there. To see them, build with a release-style version: `SCORCH_BUILD_VERSION=0.1.0 make run`. Don't press Update
in that build: it would replace the binary in `target/` with the release.

Example, checking a layout in Ukrainian without touching your own profile:

```text
mkdir -p /tmp/s/scorch
echo '{"settings":{"language":"Ukrainian"},"intro_seen":true}' > /tmp/s/scorch/profile.json
XDG_DATA_HOME=/tmp/s scorch --screen shop --shot shop.png
```

## CI and releases

GitHub Actions check every push and pull request, and release-plz turns Conventional Commits into versions, a changelog
and GitHub Releases with binaries for Linux, Windows and macOS. Details and the release steps:
[RELEASING.md](RELEASING.md). `tools/package.sh` packs the release files (tar.gz, AppImage, zip, macOS `.app`); the app
icon is `assets/icon/scorch.svg`. Windows release builds have no console window (`windows_subsystem`), so command line
output only shows in debug builds there.

## Code style and git hooks

- `.editorconfig` sets the basics for any editor: UTF-8, LF, final newline, no trailing spaces, 4-space indents (2 for
  TOML/YAML/JSON/Nix, tabs in the Makefile), 100 columns for Rust (rustfmt's default) and 120 for Markdown.
- Markdown is linted and rewrapped to 120 columns by [rumdl](https://rumdl.dev) (`.rumdl.toml`): `make docs` fixes,
  `make docs-check` checks. Tables and code blocks are left alone.
- Git hooks run with [prek](https://prek.j178.dev) (`prek.toml`), a fast drop-in for pre-commit. Install them once in a
  git checkout with `make hooks-install`; `make hooks` runs them all by hand.
  - On commit message: it must be a Conventional Commit (cocogitto, `tools/verify_commit.sh`).
  - On commit: ASCII-only code and config (`tools/check_ascii.sh`; docs and translations are UTF-8), whitespace and
    line-ending fixes, merge-conflict markers, large files, TOML/YAML/JSON syntax, Markdown, `cargo fmt --check` and
    clippy.
  - On push: the tests.
- On NixOS, prek and rumdl come from `shell.nix`, and the `make` targets run them inside `nix-shell`.

## Rules in numbers

The player's guide explains the rules; these are the exact values.

### Physics

- Launch speed = power x 0.75 px/s. Physics runs in fixed steps of 1/120 s.
- Gravity: Low 180, Normal 300, High 450 px/s².
- Wind maximum: Off 0, Calm 20, Normal 40, Strong 70. It starts at a random value within 60% of the maximum; if it
  changes, it drifts by up to 35% of the maximum each turn.
- Walls: Open loses a shell 200 px past the edge; Bouncy keeps 80% of the sideways speed.

### Damage

- A blast hits every tank within `radius + 12` px: full damage at the centre, down to 30% at the edge.
- Falling: the first 15 px are free, then 0.4 health per px.
- Health limits power: max power = health x 10, at least 50.
- Death blast: radius 35, damage 40, plus 15% of the radius and 10% of the damage of every carried shell (not Baby
  Missiles or Dirt Bombs), capped at radius 110 and damage 120. One Nuke aboard: radius 48, damage 50.
- Self-destruct: radius 75, damage 80, then the death blast.
- Sudden death: 5 health per turn, past shields. The setting is shots per tank (8/12/16); `Battle::new` multiplies it by
  the number of tanks into the turn it starts, which saved battles store.
- Kills from a death blast go to whoever caused the death, or to the dead tank itself if nobody did.

### Bots

- Aim: about 2000 simulated shots through the shared physics (`world::step_shell`), closest landing to the target wins.
- First error at a new target: random 40-100% of E, either sign.
- Next error = error x L + wobble, wobble uniform within ±12% of E. Typical miss on shot n = 0.7 x E x L^(n-1); "on
  target" means under 2° and 30 power, which is what the custom bot editor and the guide's table use.
- Presets: Moron E = 25° / 300, L = 0.8; Shooter 12° / 180, 0.5; Sniper 6° / 90, 0.3.
- Weapon: 60% chance of its biggest weapon (not Dirt Bomb), otherwise a Baby Missile.
- Target score for a new target: nearness (weight 1.0) + damage taken (0.8) + share of the most wins (0.8) - 0.5 per
  other bot aiming at that tank + random 0-0.3.

### Battle log

The feed keeps the last 6 events for 8 seconds; the full log shows the last 18 lines.

## Translations

- All on-screen text goes through `t!("key")`; the texts live in `assets/i18n/en-GB.ftl` and `assets/i18n/uk.ftl`
  ([Fluent](https://projectfluent.org), compiled into the game).
- English uses British spelling.
- Plurals use Fluent selectors (Ukrainian has one / few / many).
- Arguments `you` / `target_you` are "yes" for the solo human, for grammar.
- Player names are stored in English in saves ("You", "Player 2", "Alpha [Moron]") and translated for display
  (`Player::label`), so a save works in any language.
- Tests check that both files have the same keys, that every key used in the code exists, and that both guides mention
  every weapon and setting value in their language.
- Adding a language: copy `en-GB.ftl` to `<code>.ftl` and translate it; add a variant to `Language` in `i18n.rs`, a
  bundle for it, a `Language-<Variant>` key in every file (the name in its own language), a detection rule in
  `i18n::detect`, and a `docs/GUIDE.<code>.md` with a guide test; check that the font has the script's letters.
- The language is detected from the OS on the first run (`i18n::detect`) and then saved; an unknown saved value is
  detected again.

## Screenshots

The README's screenshots in `docs/screenshots/` come from `--screen <name> --shot <file>` (see above); regenerate them
when screens change.

## Font

`assets/fonts/ScorchMono.ttf` is Terminus with standard Cyrillic в, д, и, й. See `assets/fonts/README.md` for where it
comes from and how to rebuild it (`tools/patch_font.py`).

## Save files

JSON in the platform data folder (`~/.local/share/scorch/` on Linux):

- `profile.json`: settings (including language and update checks), last setup, custom bot, stats, whether the intro was
  shown.
- `run.json`: the game in progress; written at the start of every turn and when the shop opens, deleted when the game
  ends or is abandoned.
- `crash-<time>.log`: crash reports. All unseen ones are offered in one notice (and copied together, newest first); then
  each is renamed to `crash-<time>.seen.log`.

New saved fields need `#[serde(default)]` so older files still load.

## Versioning

Versions follow [SemVer](https://semver.org): `MAJOR.MINOR.PATCH`, with build details after a `+` (SemVer "build
metadata").

- The number lives in `Cargo.toml` and is the only part edited by hand.
- While below 1.0: MINOR goes up for new features or save-format changes that break old saves, PATCH for fixes only.
- A release: bump `Cargo.toml`, commit, tag `vX.Y.Z` (for example `v0.1.0`).
- `build.rs` adds git details when compiling:

| Build | Version |
|---|---|
| Exactly the release tag `v0.1.0`, no local changes | `0.1.0` |
| 3 commits after the tag | `0.1.0+3.gabc1234` |
| ...with uncommitted changes to tracked files | `0.1.0+3.gabc1234.dirty` |
| No release tag yet | `0.1.0+gabc1234` (`.dirty` likewise) |
| Not built from a git checkout | `0.1.0+nogit` |

`abc1234` is the short commit id; the `g` prefix is git's convention. New files git doesn't track yet don't make a build
dirty.

## Code map

| File | What |
|---|---|
| `main.rs` | Screens state machine, menus, intro, About, pause, saving flow, `--screen` |
| `battle.rs` | Turns, physics, damage, bots' aiming and targeting, battle log |
| `world.rs` | World settings, shell physics (`step_shell`), sky |
| `terrain.rs` | Heightmap terrain, terrain types, craters |
| `setup.rs` | New game screen and match rules |
| `settings.rs` | Settings screen |
| `ai.rs` | Bot profiles, aim math, difficulty budget, ladder, custom editor |
| `economy.rs` | Money rules |
| `weapons.rs` | Weapon table |
| `shop.rs` | Shop screen, bot shopping |
| `stats.rs` | All-time stats |
| `crash.rs` | Version string, crash reports |
| `save.rs` | Save files |
| `update.rs` | Self-update: check, signed download, replace, restart (see RELEASING.md) |
| `i18n.rs` | Languages, `t!` translation lookup, language detection |
| `ui.rs` | Widgets, keyboard focus, scaling to the window, font |
| `logo.rs` | Main menu logo |
| `choice.rs` | Helper for settings with a fixed list of values |
