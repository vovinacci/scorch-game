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
