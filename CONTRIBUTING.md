# Contributing to Scorch

Thanks for helping! Bug reports, ideas, translations and code are all welcome.

## Reporting bugs and ideas

Use the issue forms: **Bug report** (include the version from the main menu and, after a crash, the report the game
offers to copy), **Idea**, or **Translation**.

## Translations

All on-screen text lives in [`assets/i18n/`](assets/i18n/): `en-GB.ftl` (English) and `uk.ftl` (Ukrainian), in the
[Fluent](https://projectfluent.org) format.

- **Fixing a text:** find the line, change the text after `=`, open a pull request (or an issue with the suggestion).
- **A new language:** open a Translation issue first. Then copy `en-GB.ftl`, translate it, and the code side is
  described in [DEVELOPMENT.md](docs/DEVELOPMENT.md#translations). The player's guide gets a copy in the new language
  too (`docs/GUIDE.<code>.md`).

## Code

Start with [DEVELOPMENT.md](docs/DEVELOPMENT.md) (tools, building, the rules in numbers, code map) and
[AGENTS.md](AGENTS.md) (the project rules, for people and coding agents alike).

1. Fork, create a branch, make the change.
2. Run `make hooks-install` once; the hooks then check every commit. Before pushing: `make hooks` and `make test`.
3. Changes players can notice update both player's guides; new on-screen text goes into both `.ftl` files.
4. Open a pull request. Its title becomes the commit on `main`, so it must be a
   [Conventional Commit](https://www.conventionalcommits.org): `feat: ...`, `fix: ...`, `docs: ...`, optionally with a
   scope like `feat(bots): ...`. The changelog is generated from these titles.

By contributing you agree that your work is released under the [MIT License](LICENSE).
