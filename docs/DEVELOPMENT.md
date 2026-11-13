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
| `--screen <name>` | Open one screen directly with a fresh game: intro, menu, about, setup, custom, settings, pause-settings (settings opened from the pause menu), stats, crash, unreadable (the notice about a save file that couldn't be read), battle, pause, watch (you flee and the bots play the round out, with sound), over (a round just won), defence (every tank with a shield), buried (a Dirt Bomb on every tank), arsenal (every tank with every weapon), log, shop, abandon, new-game (the warning before replacing a saved game), final. Saving is off. |
| `--shot <file.png>` | With `--screen`: save a screenshot after a moment and quit. The name must end in `.png` and its folder must exist, or the game says so and exits with status 2 before opening the window |
| `--keys <keys>` | With `--screen`: press menu keys before the screenshot, one every 4 frames: a comma-separated list of up, down, left, right, enter (`--screen shop --keys down,down,right --shot shop.png`) |

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
- The bottom edge (y 720) is solid under the field: a shell that would fall below it is stopped on it, where the ground
  test (ground is never lower than 720) makes it land. Beside the field, with Open walls, it falls out of the world.

### Menu keys

The arrow keys move the focus along rows and columns of widgets. A widget is "that way" only if its near edge is past
the middle of the focused one (a wide button below isn't to the right), and "in line" if the two overlap across the way.
Greyed-out buttons are skipped. A section is a panel: widgets drawn inside the same `ui::panel` belong together. In
order, the focus goes to:

1. the nearest widget that way, in line, in the same section;
2. Up/Down only: the nearest widget that way in the same section (distance plus 3 times the gap across);
3. the nearest widget that way, in line, in another section;
4. Up/Down only: around to the far end of the column;
5. Up/Down only: the next or previous widget in drawing order.

Left/Right never leave the row. On a setting (a selector) they change its value instead. A new screen starts on its
first widget that works. Hovering doesn't focus a greyed-out button, but a focused button that greys out (bought, and
now too dear) keeps the focus until you move away; it just can't be pressed. When a screen shows or hides widgets
(switching the ladder on swaps three rows for a card), the focus follows the widget it was on, found again by its place
on screen.

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

The ground is painted once into a full-screen texture. After that the game compares the ground and the burn marks with
what the picture was painted from, each frame, and paints and uploads again only the columns that changed (a crater, a
trench, a mark fading): well under a millisecond for a crater, against 5 to 6 for the whole picture. Each terrain type
has a palette of top edge, topsoil and deep earth:

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
  that rim. A napalm drop leaves a burn mark of radius 8 where it burns out; burning the same spot again (same size,
  within 8 px or the radius if that is less) makes that mark fresh instead of adding one, so one napalm leaves at most 5
  marks. Each new turn a mark keeps 70% of its heat, so it's gone after 6 turns; there are at most 60 at a time. There
  is no grass where the surface is within the blast radius + 4 px of a mark's centre until it's gone, so a blast high in
  the air leaves the grass below alone. A Dirt Bomb removes the marks whose centre it lands on: fresh dirt isn't burnt.
  Marks are saved with the terrain.
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
- Balance: `make balance` prints its tables (ignored tests in `src/battle/tests/balance.rs`; they assert nothing, and
  give the same numbers on every run):
  - `balance_table` fires every weapon 300 times per aim through the real physics (perfect aim, about 1 degree / 15
    power off, about 3 degrees / 45 off) in three scenarios: one tank 700 px away on flat ground, three tanks 60 px
    apart, one tank at the bottom of a valley. The aim used is the one, among those whose first damage lands within 150
    px of the middle target, that does the most damage with close aim. Each shot is played out like a turn (falls and
    death blasts included) and a tank loses at most its 100 health. It prints mean damage to the targets, kill% and
    self-damage (rough aim) and damage per dollar (close aim). Most weapons give 0.3-0.5 damage per $ on one target and
    about twice that in a crowd; the Missile (about 1.1) is the cheap workhorse. Bots fire the weapon with the highest
    price per shot, so a price change also changes which weapon a bot prefers.
  - `balance_order` plays 600 rounds of equal bots with the starting weapons for each line: the share of rounds won by
    firing order, the mean number of turns, and the share of rounds that reached sudden death.
  - `balance_buried` puts a tank under a direct Dirt Bomb hit and fires each weapon straight up until the barrel is
    clear: shots taken and health left.
  - `balance_deflector` sends one Missile at a full Mag Deflector on five paths: energy and health left.
  - `balance_types` plays the same whole games against each bot type, alone and with two Morons, to set the types'
    threat points. About ten minutes.
  - `balance_difficulty` plays them against line-ups drawn for each difficulty and one to three bots, to set the
    budgets. About ten minutes.
  - `balance_shots` does the same for the types that aim differently (Tosser, Poolshark) behind Open, Bouncy and
    Wrap-around walls. About a quarter of an hour.
  - `balance_batteries` gives the stand-in shields or batteries for nothing, each round or once for the game, and tries
    three health limits for using them: what a battery is worth and when to use it. About seven minutes.
  - `balance_ladder` and `balance_ladder_sniper` play whole games (5 rounds, default settings, everyone shopping between
    rounds as bots do) of a stand-in for the player against 38 line-ups of one to three bots, 200 games each: the share
    of games it wins outright, the share with a shared first place, and its mean round wins. The stand-in is a Shooter
    or a Sniper whose aim the bots can't see, as they can't see a human's. These two take about a quarter of an hour
    each.

### Parachutes, shields and batteries

- Items live in `src/items.rs`: Parachute $40 x2, Shield 50 energy $120, Heavy Shield 100 $200, Mag Deflector 40 $150,
  Battery 25 health $60.
- Battery (`Battle::use_battery`): the **B** key while a human aims, as often as there are batteries; +25 health up to
  100, not used at full health; the turn goes on. No switch in the inventory (`Item::automatic`). A bot uses one at the
  start of its turn while its health is 75 or less (`BOT_BATTERY_HEALTH`), so none of a battery is wasted.
- The price and the bots' rule come from `balance_batteries` (whole games of a Shooter-like stand-in with no shields of
  its own, share of games won outright against one / three Shooters): nothing 36 / 9%; a Shield each round 79 / 30%; a
  Heavy Shield each round 99 / 54%; 1 battery each round 62 / 18%; 2 batteries 86 / 37%; 4 batteries 98 / 53%. So two
  batteries are worth about a Shield and four a Heavy Shield, and a battery costs half a Shield. Four batteries for a
  whole game, used once health is at or under 75 / 50 / 25: 76 / 58 / 36% against one Shooter, 22 / 16 / 12% against
  three. Waiting costs more than a wasted point or two saves: a tank kept low shoots short and dies to the next hit with
  batteries left.
- Round start: the switched-on shield with the most energy is raised and used up (`items::shield_to_raise`).
- A blast's damage to a tank goes to its shield first; the rest to health. Damage pay and stats count shield damage too,
  so shields don't shut down the catch-up money.
- Mag Deflector: enemy shells within 80 px of the tank's centre get an acceleration of 5000 px/s^2 away from it, fading
  linearly to 0 at 80 px. The tank's own shells aren't affected. Bots' aim simulation includes deflectors (`deflect` is
  shared), so they aim around them. A shell pays 10 energy for each pass through the field: when it comes in, and again
  if it gets 6 px clear of the field and comes back (a bit per tank in `Projectile::charged`, cleared on the way out and
  at every Leapfrog hop). A deflector at 0 takes no new passes but keeps pushing the shells that paid until they leave,
  absorbs no damage, is drawn only as a faint ring while a shell that paid is still in its field, and goes down when the
  turn's shells are done or at the next turn (`drop_spent_deflectors`).
- That push turns most small shells lobbed from above away: of the shells that would hurt a tank with no deflector, 5 of
  68 Baby Missiles, 21 of 84 Missiles and 43 of 105 Baby Nukes still do (with 3000 it was 32, 63 and 91).
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
- Types (`ai::PRESETS`): Moron E = 25° / 300, L = 0.8; Shooter 12° / 180, 0.5; Sniper 6° / 90, 0.3; Spoiler 9° / 135,
  0.4; Cyborg 3° / 45, 0.2; Tosser as a Shooter; Poolshark as a Spoiler; a Chooser has no aim of its own. The types keep
  their places in the list: the ladder's rungs are indices into it.
- A Spoiler (`Targets::Leader`) shoots at the living enemy with the most round wins, the nearer of two level. No
  revenge, no penalty for a crowded target, no three-bot rules; it keeps its learned aim only while that aim is on the
  leader. While no living enemy has a round win it chooses as the others do. A custom bot always chooses as the others
  do.
- A Tosser (`Shots::Lobs`) searches only angles 61-121° (`LOB_ANGLES`); if its best lob lands more than 60 px from the
  target (`LOB_NEAR`) it searches all angles like the others. A Poolshark (`Shots::Walls`) takes the closest shot that
  came off a Bouncy wall or round a Wrap-around one (`Landing::off_wall`) if it lands at most 20 px further from the
  target than the closest shot of all (`WALL_SHOT_SLACK`); behind Open or Solid walls no shot does.
- A Chooser plays each round as one of the other seven types, picked at random when the round is made
  (`AiProfile::for_round`, kept in `Tank::role` and saved with the battle). Its name on screen stays Chooser.
- Threat points (`ai::THREAT`, kept in halves): Moron 1, Tosser 1.5, Shooter 2, Poolshark and Chooser 2.5, Sniper and
  Spoiler 3, Cyborg 4. A line-up starts everyone as a Moron, moves random bots up to the next dearer cost while the
  budget lasts, then makes each bot a random type of its cost; the budget is always spent exactly. A lone bot is never a
  Spoiler: alone it is only a worse shot than a Sniper.
- The costs come from `balance_types` (whole games, as `balance_ladder`). Share of games won outright by a Shooter-like
  and a Sniper-like player against one bot: Moron 100% / 100%, Tosser 64 / 95, Shooter 46 / 92, Chooser 36 / 84, Spoiler
  25 / 81, Poolshark 23 / 75, Sniper 7 / 50, Cyborg 1 / 23. With two Morons beside it: Tosser 46 / 78, Shooter 28 / 68,
  Poolshark 26 / 49, Chooser 24 / 60, Spoiler 13 / 41, Sniper 11 / 32, Cyborg 4 / 23. A Spoiler is worth a Sniper only
  in company, where it hunts the player who leads. `balance_shots` puts a Tosser beside a Shooter and a Poolshark beside
  a Sniper behind Open, Bouncy and Wrap-around walls: the walls change little.
- Difficulty budgets (`ai::BUDGETS`, threat points for 1 / 2 / 3 bots): Easy 1 / 2 / 3, Normal 2 / 3 / 4, Hard 3 / 4 /
  6, Brutal 4 / 6 / 9. `balance_difficulty` plays line-ups drawn as the game draws them: a Shooter-like player wins
  about 97 / 70 / 59% of games outright on Easy, 51 / 34 / 29% on Normal, 9 / 12 / 18% on Hard and 1 / 6 / 7% on Brutal;
  a Sniper-like one 100 / 95 / 90%, 91 / 73 / 68%, 48 / 27 / 43% and 23 / 18 / 19%. With two bots many games end in a
  shared first place, which is why Hard with two looks harder than with three.
- Weapon: 60% chance of its weapon with the highest price per shot (`Weapon::shot_price`) that isn't a utility one,
  otherwise a Baby Missile. Shots landing closer to the bot than the weapon's radius + 25 px are ruled out; for
  MIRV-type weapons add the outer warhead's drift: its sideways speed x the fall time from the top of the arc (landing
  vertical speed / gravity).
- Buried (the muzzle is under the ground at every angle the bot tries): it fires straight up to dig out. With a Riot
  Bomb if it has one (no damage). Otherwise with the blast weapon that clears the muzzle in one shot (twice its radius
  is more than the dirt above the muzzle) for the least health, if that costs at most 40% of the health it has left: a
  Missile costs about 39 and gets it out of a direct Dirt Bomb hit. Otherwise with Baby Missiles: about 15 health and 40
  px of dirt a shot, two shots for a direct hit.
- Shop: first a pack of Riot Bombs if it has none and can pay for it (digging out without one takes two turns, enough
  for a player to keep a bot buried for good); then 8 random picks among the weapons it fires (the non-utility ones and
  the Riot Bomb) and the items, each bought if affordable.
- Target score for a new target: nearness (weight 1.0) + damage taken (0.8) + share of the most wins (0.8) - 0.5 per
  other bot aiming at that tank + random 0-0.3.
- With exactly three tanks left, all bots: a further - 0.5 for the tank that aims at the bot choosing, and no revenge on
  a tank the third bot already aims at. Without this the bot firing first wins 24% of rounds among three equal bots
  instead of a third (the others avoid its target, which leaves only itself, and whoever it hits takes revenge too);
  with it, 36-38%, about the edge the first shot gives with two tanks (`balance_order`). With a human or four tanks
  alive the rules don't apply.

### Ladder

- Ten rungs (`ladder::RUNGS`), each a fixed line-up of bots; the Bots and Difficulty settings aren't used. The order
  comes from `balance_ladder` (whole games, see Weapons), which plays 38 line-ups a rung could be made of: against the
  ten chosen a Sniper-like player wins 100, 97, 95, 92, 84, 68, 43, 31, 27 and 17% of games outright, a Shooter-like one
  100, 73, 64, 46, 36, 28, 20, 12, 7 and 5%. Every bot type has a rung. More bots is not always harder, because they
  also fight each other: three Morons (60% for a Shooter-like player) are easier than one Shooter.
- Only with one human player; with two the switch does nothing.
- After a game: more round wins than the best bot is a win (up a rung; on rung 10, champion + 1 and stay); the same
  number is a shared first place (no change); fewer is a loss. Two losses in a row on a rung go down one and start the
  count afresh; a win clears the count, a shared first place doesn't; rung 1 has no way down.
- Abandoning a ladder game is a loss, also when a new game replaces a saved one.
- A game with a path preview (Short path, Full path) is on the ladder's rung but isn't recorded, won or lost.
- The player's place (rung, losses in a row, best rung, champion count) is `Profile::ladder` in `profile.json`, apart
  from the setup, so switching the ladder off and on keeps it.

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
change. Take them fullscreen (`"fullscreen": true` in the profile) on a 4K screen, with `"turn_order": "RoundRobin"` in
the profile's `setup` so the battle shot is your turn. Scale them down to 1280x720
(`magick <file> -filter Box -resize 1280x720 <file>`, an exact 3 to 1), reduce them to a palette
(`pngquant --quality 85-98 --speed 1 --strip`; the grain in the ground makes full-colour shots several times bigger) and
shrink them with `oxipng -o 4 --strip safe`.

## Sound delay and the patched audio library

macroquad plays sound through the quad-snd crate, which mixes a buffer of 4096 frames at a time; the sound device holds
one more. A sound therefore started 0.1 to 0.2 s after its event, though the game plays it in the same frame. The
repository carries quad-snd 0.2.8 in `third_party/quad-snd` with the Linux (ALSA) buffer at 1024 frames, a quarter of
that, and `Cargo.toml` puts it in place of the published crate with `[patch.crates-io]`. `third_party/README.md` says
what was changed and what to do when macroquad updates. The macOS and Windows buffers are as published. Without the
patch the delay is clearly back (tried by ear), so the copy stays until quad-snd makes the buffer a setting.

Bluetooth headphones and some USB sound devices go to sleep while nothing plays and take a moment to wake: the start of
the next sound is cut, short sounds are lost, the rest comes late. So while sound is on, the game loops a 10 Hz hum at
1% of full level (`Sounds::keep_awake`): too low to hear, loud enough to survive the system's volume control, a whole
number of waves a second so the loop has no click. With sound Off it stops.

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

Both files carry `"version"` (`save::FORMAT`, now 1; files from before it count as 0). Raise it with a change an older
version of the game can't read. A file that doesn't parse, or has a higher version than the game knows, is renamed to
`profile.unreadable.json` or `run.unreadable.json` (replacing an earlier one) before anything is saved again, and the
main menu shows a notice with that path until it is dismissed. With `--screen` nothing is renamed.

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
| `main.rs` | Start-up: the command line, the window and its icon; the tests that the guides name every weapon and setting |
| `game.rs` | The game around the battles: the screens and how one leads to the next (`Game::step`), starting and ending games and rounds, saving, the main loop |
| `game/menus.rs` | Main menu and its notices, pause menu, final standings, intro, About, the confirmation dialog |
| `game/dev.rs` | Developer aids: `--screen`, `--keys`, the screenshot's game area |
| `battle.rs` | One round: the tanks, turns and phases, aiming and firing, falling and settling, the battle log; the numbers behind the rules |
| `battle/shells.rs` | Shells: flight, what happens where they land, fire, Mag Deflectors, blasts and damage; the flight simulation bots and the aiming help share |
| `battle/bots.rs` | Bots: whom they shoot at, what with, how they find the aim and dig out |
| `battle/coach.rs` | The coaching advice of the aiming help |
| `battle/draw.rs` | Drawing the battle: scene, HUD, tanks, flames, weapon strip, full battle log |
| `battle/tests.rs`, `battle/tests/` | The battle's unit tests: shared helpers, then one file each for rules, weapons, shields, bots, coaching, what is on screen, and the balance tools (other files keep their tests in a `tests` module at the bottom) |
| `world.rs` | World settings, shell physics (`step_shell`), sky |
| `terrain.rs` | Heightmap terrain, terrain types, craters |
| `setup.rs` | New game screen and match rules |
| `settings.rs` | Settings screen |
| `ai.rs` | Bot profiles, aim math, difficulty budget, ladder, custom editor |
| `economy.rs` | Money rules |
| `weapons.rs` | Weapon table |
| `items.rs` | Parachutes, shields and batteries: item table, which shield goes up |
| `ladder.rs` | The ladder: its ten rungs, the player's place on it, what a won, shared or lost game does |
| `shop.rs` | Shop screen, bot shopping |
| `stats.rs` | All-time stats |
| `crash.rs` | Version string, crash reports |
| `rng.rs` | Random numbers: one generator per thread, seeded from the clock in the game, in its fixed starting state in every test (so tests repeat exactly and don't disturb each other; `rng::seed` starts a sequence over) |
| `save.rs` | Save files |
| `update.rs` | Self-update: check, signed download, replace, restart (see RELEASING.md) |
| `i18n.rs` | Languages, `t!` translation lookup, language detection |
| `ui.rs` | Widgets, keyboard focus, scaling to the window, font |
| `sound.rs` | 8-bit style sound effects (pulse, triangle and noise voices) synthesised at start, no sound files; played from the battle's `take_sounds()`. To listen while tuning: `SCORCH_SOUNDS=<dir> cargo test write_sounds -- --ignored` |
| `third_party/` | Other people's crates the game needs a change in (see its README); not linted or formatted with the game |
| `logo.rs` | Main menu logo |
| `choice.rs` | Helper for settings with a fixed list of values |
