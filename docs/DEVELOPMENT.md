# Scorch: development notes

For people working on the game. The player's guide is `GUIDE.md` (and its Ukrainian copy `GUIDE.uk.md`); keep both in
sync with any change players can notice. This file holds what players don't need: how to build and test, the exact
numbers behind the rules, file formats, versioning and the code map.

## Tools

- **Rust** is pinned with [mise](https://mise.jdx.dev) in `mise.toml` (1.99.1, with rustfmt and clippy). With mise
  installed, entering this folder selects it (`mise trust` once, `mise install` if missing). CI uses the same file.
  Without mise, any Rust 1.88 or newer builds the game.
- **Python 3.15** for `tools/patch_font.py` (it uses lazy imports); the other Python tools run on any Python 3. On NixOS
  `make tools` builds `.venv` with the 3.15 from `shell.nix`; CI only needs the tools, so it uses the runner's.
  **Python tools** (prek for git hooks, rumdl for Markdown, fonttools for the font) are pinned with hashes in
  `requirements.txt`, and the Linux release build tools (zig, cargo-zigbuild) in `requirements-build.txt`; `make tools`
  installs them into `.venv`, and the other `make` targets use them from there. CI installs the same file, and
  Dependabot keeps it current.
- **NixOS:** `shell.nix` provides ALSA, the graphics libraries the game loads at run time, Python and cocogitto (commit
  message check); the `make` targets use it automatically. Elsewhere install cocogitto with `cargo install cocogitto`.

## Binaries

- **Linux:** links glibc (and libgcc) and, for sound, ALSA (`libasound.so.2`; building needs its headers, Debian/Ubuntu
  `libasound2-dev`). miniquad loads `libX11.so.6`, `libXi.so.6`, `libxkbcommon.so.0` and `libGL.so.1` at run time
  (default X11 backend; Wayland desktops use XWayland), so the game uses the player's own GPU driver; a fully static
  (musl) build couldn't load it. Release builds need glibc 2.35 or newer, whatever the build machine has:
  `tools/build-linux.sh` links against glibc 2.35 with cargo-zigbuild and fails if the binary needs anything newer. On
  distributions these libraries are part of every desktop (Debian/Ubuntu:
  `libx11-6 libxi6 libxkbcommon0 libgl1 libasound2`; PipeWire and PulseAudio need ALSA too). Without a working sound
  device the game runs silently.
- **NixOS:** `flake.nix` packages the game with alsa-lib and wraps it with those four libraries; `nix build` builds it
  (and runs the tests), `nix run .` plays it, players use `nix run github:vovinacci/scorch-game`. The version comes from
  the flake's commit (`SCORCH_BUILD_VERSION`, see `build.rs`). CI builds the flake; a weekly workflow updates
  `flake.lock` (Dependabot doesn't handle Nix).
- **Windows:** the C runtime is linked statically (`.cargo/config.toml`), so no Visual C++ redistributable is needed.
- **macOS:** links only system frameworks; one universal binary for Intel and Apple Silicon.

## Building and running

```text
make           # list all targets
make run       # debug build, then play
make release   # optimised build, then play
make test      # unit tests
make lint      # clippy on code and tests, warnings are errors
```

On NixOS, `make` runs cargo inside `nix-shell` (see `shell.nix`). The window may be any size; all layout uses a 1280x720
virtual screen, scaled to fit.

Command line options:

| Option | What it does |
|---|---|
| `--version`, `-V` | Print the version and exit |
| `--crash-test` | Crash on purpose, to check crash reports |
| `--update` | Check for a new version and install it from the command line (same checks as the menu) |
| `--screen <name>` | Open one screen directly with a fresh game: intro, menu, about, setup, custom, settings, pause-settings (settings opened from the pause menu), stats, crash, battle, pause, over (a round just won), defence (every tank with a shield), arsenal (every tank with every weapon), log, shop, abandon, new-game (the warning before replacing a saved game), final. Saving is off. |
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
icon is `assets/icon/scorch.svg`, rendered to the PNGs and `scorch.ico` beside it (after editing the SVG, render them
again). The game sets its window, taskbar and Dock icon from the 16, 32 and 64 px PNGs (otherwise the OS shows
miniquad's logo), and `build.rs` embeds `scorch.ico` in the Windows `.exe`. Windows release builds have no console
window (`windows_subsystem`), so command line output only shows in debug builds there.

## Code style and git hooks

- `.editorconfig` sets the basics for any editor: UTF-8, LF, final newline, no trailing spaces, 4-space indents (2 for
  TOML/YAML/JSON/Nix, tabs in the Makefile), 100 columns for Rust (rustfmt's default) and 120 for Markdown.
- Markdown is linted and rewrapped to 120 columns by [rumdl](https://rumdl.dev) (`.rumdl.toml`): `make docs` fixes,
  `make docs-check` checks. Tables and code blocks are left alone.
- Git hooks run with [prek](https://prek.j178.dev) (`prek.toml`), a fast drop-in for pre-commit. Install them once in a
  git checkout with `make hooks-install`; `make hooks` runs them all by hand.
  - On commit message: it must be a Conventional Commit (cocogitto, `tools/verify_commit.sh`).
  - On commit: ASCII-only code and config (`tools/check_ascii.sh`; docs and translations are UTF-8), whitespace and
    line-ending fixes, merge-conflict markers, large files, TOML/YAML/JSON syntax, Markdown, shell scripts (shfmt and
    shellcheck; style in AGENTS.md), Python (ruff, every rule on, exceptions in `ruff.toml`), `cargo fmt --check` and
    clippy.
  - On push: the tests.
- On NixOS, prek and rumdl come from `shell.nix`, and the `make` targets run them inside `nix-shell`.
- Lints live in `[lints]` in `Cargo.toml`, so cargo, editors and CI share them:
  - Clippy's `pedantic` group, except three cast lints (pixel and count conversions are everywhere in game code).
  - `unsafe_code` is forbidden. `rust_2018_idioms` and `unused_qualifications` are on.
  - From the restriction group: no `dbg!`, `todo!` or `unimplemented!`, and every exception is an
    `#[expect(lint, reason = "...")]`: `#[allow]` is refused, and an `#[expect]` that no longer applies is an error.
  - `clippy.toml` lists product names that `doc_markdown` should not put in backticks.
- Dependencies are checked by [cargo-deny](https://embarkstudios.github.io/cargo-deny/) (`deny.toml`, CI job
  "Dependencies"; locally `cargo deny check`): security advisories, licences (a short allow list, plus per-crate
  exceptions), duplicate versions (warning) and sources. Ignored advisories are listed there with a reason.

## Rules in numbers

The player's guide explains the rules; these are the exact values.

### Physics

- Launch speed = power x 0.75 px/s. Physics runs in fixed steps of 1/120 s.
- Gravity: Low 180, Normal 300, High 450 px/s².
- Wind maximum: Off 0, Calm 20, Normal 40, Strong 70. It starts at a random value within 60% of the maximum; if it
  changes, it drifts by up to 35% of the maximum each turn.
- Walls: Open loses a shell 200 px past the edge; Bouncy keeps 80% of the sideways speed.

### Menu keys

The arrow keys move the focus to the nearest widget on screen that way. A widget counts as "that way" only if its near
edge is past the middle of the focused one (a wide button below isn't to the right). Of those, the one with the smallest
distance that way plus 3 times the gap across (0 for widgets in the same row or column) wins. Up and Down go to the next
or previous widget in drawing order when nothing is that way. On a setting (a selector), Left and Right change its value
instead. Greyed-out buttons never take the focus: the keys skip them, hovering doesn't focus them, and if the focused
one greys out the focus goes to the nearest working one the way the keys last moved, or else to the closest working one.

### Mouse aiming

- Left button pressed on the battlefield (below the 44 px HUD bar, not on a strip box) starts a drag; while held,
  angle = direction from the barrel base to the cursor (below the barrel: flat 0 or 180) and power = distance x 2,
  capped at the tank's maximum. With Shift held, changes from where Shift went down count x 0.2. Release keeps the aim.
- Wheel: power +-10 per notch (+-1 with Shift). Right button fires, like Space.

### Aiming help

- `Setup::aim_help` (saved in `Rules`, so a battle keeps it): Off, Coaching, Short path, Full path. Only human turns get
  it; "I'm feeling lucky" leaves it as it is.
- Preview: `predict_path` flies the current aim through the same physics as a shot (wind, walls, enemy Mag Deflectors;
  tanks ignored) and draws every 4th step as a dot, the first third of the steps (Short) or all (Full). The path is kept
  while turn, aim and wind stay the same.
- Coaching (any level but Off): when a human fires, `fire` works out where the shell first comes down (the unsplit shell
  for a MIRV, the first touch for a Roller, Digger or Leapfrog). If the shot hurts no enemy, `coach` picks the enemy it
  was meant for: the nearest to that point (the short way round with wrap-around walls; off the field, the farthest that
  way). At the shooter's next turn, `advise` searches the powers it can use (5 apart) at its current angle in that
  turn's wind: within 40 px of the enemy, the gap to its power gives "a little" (up to 25), plain (up to 75) or "a lot"
  more or less, or "about right" under 8; otherwise the angles 10-170 (5 apart) say whether to aim higher or lower, or
  that nothing within its power reaches (mentioning damage when "Health limits power" cut it). The advice goes into the
  full log and stays above the weapon strip (`Tank::coaching`) until the tank fires.
- A shot is a miss when it hurts no enemy and drops none (`shot_hit`), whatever else gets logged that turn.

### Random world settings

- Terrain, Wind, Gravity, Walls and Sky can be Random. `Choice::roll` (a value whose key ends in `-Random` becomes a
  real value from the rest of the list, wherever Random sits) is applied by `World::for_round` in `Battle::new`. The
  battle keeps both worlds (`setup_world`, `None` in older saves, and `rules.world`); `Battle::worlds` gives the pair.
  Code that meets an unresolved Random (only the setup preview) treats it as Normal, Open walls or Night.
- The round's picks: the full list as the first battle log line; gravity and walls as one line each under the HUD's turn
  counter (at most 300 px, right-aligned, clear of the centred sudden death banner); in the pause menu's "This game" the
  picked values are blue, with a note under the list.
- "I'm feeling lucky" sets world options with `Choice::any_real`, never Random.

### Terrain look

The ground is painted once into a full-screen texture and repainted only when it changes (a crater, a flattened tank
pad, a landslide). Each terrain type has a palette of top edge, topsoil and deep earth:

- The top 3 px are grass (a sun-bleached sandstone rim in the canyon), except on slopes steeper than 1.3 (bare earth
  there).
- Below, topsoil blends into deep earth by height on screen (y 380 to 720), not by depth, so cliffs and craters cast no
  dark columns into the earth. The 14 px under the surface are lighter, fading out with depth.
- Snow caps the mountain peaks: 0.4 px of snow for every pixel the surface rises above a snow line that waves gently
  around y 290, at most 22 px.
- The canyon has rock layers built from 9 px slices in five close shades (93% to 105%), so layers come out uneven in
  thickness; their lines wave gently and tilt slightly across the screen.
- Every pixel gets a little fixed grain (up to 4% lighter or darker).
- Every blast (also a tank blowing up; not Dirt Bomb, Riot Bomb or Digger) leaves a burn mark: the earth within 6 px +
  half the radius outside the crater blends towards charred brown, up to 85% at the crater's edge and fading out over
  that rim. A napalm drop leaves a burn mark of radius 4 where it burns out; burning the same spot again (within the
  radius, same size) makes that mark fresh instead of adding one. Each new turn a mark keeps 70% of its heat, so it's
  gone after 6 turns; there are at most 60 at a time. There is no grass within the blast radius + 4 px of a mark's
  centre (left and right) until it's gone. Marks are saved with the terrain.
- Napalm drops are drawn as flames: a glowing patch and three layered flames (red, orange, yellow) up to 37 px high,
  flickering and swaying, with a rising spark now and then. Each drop's flame stands up to 9 px to either side of it,
  fixed per drop, so drops pooled in a dip burn as one wide fire.

The look is saved with the terrain, so a resumed game looks the same. Older saves show as Rolling hills.

### Sky scenery

Drawn every frame behind the ground, still and in fixed places, so the same sky always looks the same (also in the setup
preview and, for Night, behind the menus). Nothing in it takes part in the game.

- Night: 140 stars between y 40 and 460, more of them higher up, fading towards the horizon; one in ten is 2 px.
- Sunset: a sun of radius 40 at (900, 455), partly behind the far hills.
- Day: a sun of radius 26 at (820, 120), left of the turn panel, and four thin wispy clouds: each a few long, flat
  streaks of white, six nested ellipses at 5% each, so they fade out towards their edges.
- Suns have a glow out to three times their radius, in 16 faint steps.
- Two rows of far hills (tops around y 370 to 440 and 445 to 500) in the sky's own colour at their height, 8% and 14%
  darker.

### Damage

- A blast hits every tank within `radius + 12` px: full damage at the centre, down to 30% at the edge.
- Falling: the first 15 px are free, then 0.4 health per px.
- Health limits power: max power = health x 10, at least 50.
- Death blast: radius 35, damage 40, plus 15% of the radius and 10% of the damage of every carried shell (not Baby
  Missiles or utility weapons), capped at radius 110 and damage 120. One Nuke aboard: radius 48, damage 50.
- Self-destruct: radius 75, damage 80, then the death blast.
- Sudden death: 5 health per turn, past shields. The setting is shots per tank (8/12/16/24); `Battle::new` multiplies it
  by the number of tanks into the turn it starts, which saved battles store.
- Kills from a death blast go to whoever caused the death, or to the dead tank itself if nobody did.

### Weapons

- The table is `WEAPONS` in `src/weapons.rs`; saves store ammo by index, so new weapons go at the end.
- Balance: `cargo test --release balance_table -- --ignored --nocapture` fires every weapon 300 times per aim through
  the real physics (perfect aim, about 1 degree / 15 power off, about 3 degrees / 45 off) in three scenarios: one tank
  700 px away on flat ground, three tanks 60 px apart, one tank at the bottom of a valley. The aim used is the one,
  among those whose first damage lands within 150 px of the middle target, that does the most damage with close aim.
  Each shot is played out like a turn (falls and death blasts included) and a tank loses at most its 100 health. It
  prints mean damage to the targets, kill% and self-damage (rough aim) and damage per dollar (close aim). Most weapons
  give 0.3-0.5 damage per $ on one target and about twice that in a crowd; the Missile (about 1.1) is the cheap
  workhorse. Bots fire the weapon with the highest price per shot, so a price change also changes which weapon a bot
  prefers.
- Utility weapons do no damage: Dirt Bomb, Digger, Tracer, Riot Bomb. Bots don't buy or fire them, and they don't add to
  the death blast.
- MIRV-type weapons split when the shell starts to fall: `warheads` shells with extra sideways speeds `spread` apart,
  centred on the shell's (MIRV 5 x 45 px/s, Death's Head 9 x 30 px/s).
- Tracer is a probe: no shot in accuracy stats, no "missed" in the log, no Mag Deflector charge. It doesn't end the turn
  (`TracerTurn`): once it lands, the same tank aims again with its last real weapon, no turn passes and sudden death
  doesn't drain; a second one that turn is blocked (fired anyway, it becomes a Baby Missile). Balance: a look turns a
  rough shot into a close one, worth +15 to +39 damage depending on the weapon (0 for a Roller), against $25. Its path
  is kept in `Battle::tracers` (saved with the battle) with points thinned to 6 px apart, and drawn every phase until
  the round ends: every other 6 px segment in the shooter's colour (alpha 0.85, 1.5 px) over a dark edge (3.5 px, alpha
  0.45); segments above the screen are skipped. It doesn't replace the shooter's last real shot path (every 6th point in
  white, alpha 0.3, while aiming).
- Riot Bomb carves a circle of radius 60. When a Riot Bomb lands or a Digger finishes its trench, every enemy tank over
  the changed ground that is no longer held up (the same support test as falling: the highest ground under its 24 px
  width) gets the shooter as last hitter and revenge target, and the log says its ground gave way. So a deadly fall is
  the shooter's kill, and the shot isn't logged as a miss.
- Weapon strip: up to 7 boxes per row (at most 150 px wide), extra rows above; keys 1-9, 0, -, = for the first 12. The
  boxes are opaque. Generated ground reaches y 690 and craters go lower, so a tank whose area (its name's width, 76 px
  tall) overlaps a box is drawn again on top of the strip, body only (`draw_tank_body`).
- Over the battlefield, the log feed, the top-right lines and the coaching line sit on `backing`: the panel colour at
  40% on the Day sky, nothing on Night and Sunset (`backing_alpha`), where the text outline is enough. Aiming dots are
  `guide_dot`s with a dark edge.

### Parachutes and shields

- Items live in `src/items.rs`: Parachute $40 x2, Shield 50 energy $120, Heavy Shield 100 $200, Mag Deflector 40 $150.
- Round start: the switched-on shield with the most energy is raised and used up (`items::shield_to_raise`).
- A blast's damage to a tank goes to its shield first; the rest to health. Damage pay and stats count shield damage too,
  so shields don't shut down the catch-up money.
- Mag Deflector: enemy shells within 80 px of the tank's centre get an acceleration of 3000 px/s^2 away from it, fading
  linearly to 0 at 80 px. The tank's own shells aren't affected. Bots' aim simulation includes deflectors (`deflect` is
  shared), so they aim around them. A shell entering the field pays 10 energy once (a bit per tank in
  `Projectile::charged`); a deflector at 0 takes no new shells but keeps pushing those that paid (a Leapfrog keeps that
  across its hops), absorbs no damage, isn't drawn, and goes down when the turn's shells are done or at the next turn
  (`drop_spent_deflectors`).
- Parachute: opens once a fall passes the free 15 px (if owned and switched on), uses one, slows the fall to 90 px/s and
  cancels the fall damage.

### Turn order

`setup::turn_order` builds each round's firing order, which the battle stores. Losers first / Winners first: shuffle,
then move the first player with the lowest (highest) (wins, kills) to the front, so ties are random and the rest of the
cycle changes every round. Random: shuffle. Round robin: player order rotated by `round - 1`. The shop's "you shoot
first" note only shows when it's certain (`setup::sure_to_start`). Battles saved without an order use player order.

### Bots

- Aim: about 2000 simulated shots through the shared physics (`world::step_shell`), closest landing to the target wins.
- First error at a new target: random 40-100% of E, either sign.
- Next error = error x L + wobble, wobble uniform within ±12% of E. Typical miss on shot n = 0.7 x E x L^(n-1); "on
  target" means under 2° and 30 power, which is what the custom bot editor and the guide's table use.
- Presets: Moron E = 25° / 300, L = 0.8; Shooter 12° / 180, 0.5; Sniper 6° / 90, 0.3.
- Weapon: 60% chance of its weapon with the highest price per shot (`Weapon::shot_price`) that isn't a utility one,
  otherwise a Baby Missile. Shots landing closer to the bot than the weapon's radius + 25 px are ruled out; for
  MIRV-type weapons add the outer warhead's drift: its sideways speed x the fall time from the top of the arc (landing
  vertical speed / gravity).
- Buried (the muzzle is under the ground at every angle the bot tries): it fires straight up to dig out. With a Riot
  Bomb if it has one (no damage). Otherwise with the blast weapon that clears the muzzle in one shot (twice its radius
  is more than the dirt above the muzzle) for the least health, if that costs at most 40% of the health it has left: a
  Missile costs about 39 and gets it out of a direct Dirt Bomb hit. Otherwise with Baby Missiles: about 15 health and 40
  px of dirt a shot, two shots for a direct hit.
- Shop: 8 random picks among the weapons it fires (the non-utility ones and the Riot Bomb) and the items, each bought if
  affordable.
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

The README's screenshots in `docs/screenshots/` come from `--screen <name> --shot <file>` (see above), in English:
`arsenal` (saved as `battle.png`), `menu`, `shop`, `setup` and `custom` (saved as `custom-bot.png`). A shot holds only
the 16:9 game area, whatever the window's shape. Build with `SCORCH_BUILD_VERSION=<release>` so the corner shows a plain
version (then use a profile with `"check_updates": false`, or the menu asks about updates), regenerate them when screens
change. Take them fullscreen (`"fullscreen": true` in the profile) on a 4K screen, scale them down to 1920x1080
(`magick <file> -filter Box -resize 1920x1080 <file>`) and shrink them with `oxipng -o 4 --strip safe`.

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
| `battle/tests.rs` | The battle's unit tests (other files keep theirs in a `tests` module at the bottom) |
| `world.rs` | World settings, shell physics (`step_shell`), sky |
| `terrain.rs` | Heightmap terrain, terrain types, craters |
| `setup.rs` | New game screen and match rules |
| `settings.rs` | Settings screen |
| `ai.rs` | Bot profiles, aim math, difficulty budget, ladder, custom editor |
| `economy.rs` | Money rules |
| `weapons.rs` | Weapon table |
| `items.rs` | Parachutes and shields: item table, which shield goes up |
| `shop.rs` | Shop screen, bot shopping |
| `stats.rs` | All-time stats |
| `crash.rs` | Version string, crash reports |
| `save.rs` | Save files |
| `update.rs` | Self-update: check, signed download, replace, restart (see RELEASING.md) |
| `i18n.rs` | Languages, `t!` translation lookup, language detection |
| `ui.rs` | Widgets, keyboard focus, scaling to the window, font |
| `sound.rs` | 8-bit style sound effects (pulse, triangle and noise voices) synthesised at start, no sound files; played from the battle's `take_sounds()`. To listen while tuning: `SCORCH_SOUNDS=<dir> cargo test write_sounds -- --ignored` |
| `logo.rs` | Main menu logo |
| `choice.rs` | Helper for settings with a fixed list of values |
