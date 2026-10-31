# Scorch: GitHub, CI and releases

Status: **planned**. This is the agreed target setup; tick items off in the roadmap as they land.

## GitHub repository

- Name `scorch-game` (the game and binary stay "Scorch" / `scorch`), public, MIT.
- Description: "Turn-based artillery game, a tribute to Scorched Earth (1991). Rust + macroquad. English and
  Ukrainian."
- Topics: `game`, `artillery`, `scorched-earth`, `turn-based`, `retro`, `rust`, `macroquad`, `gamedev`.
- Social preview: the battle screenshot (1280x640).
- README badges: CI, latest release, licence.
- `main` is protected: CI and the PR title check must pass; squash merges only, so the PR title becomes the commit.
- Dependabot: weekly for Cargo and GitHub Actions.
- Issue templates: bug report (version, OS, crash report) and idea.

## Commits

[Conventional Commits](https://www.conventionalcommits.org): `type(scope)!: summary`.

| Type | Use for | In the changelog |
|---|---|---|
| `feat` | Something players or developers can use | Features |
| `fix` | A bug fix | Fixes |
| `perf` | Faster, same behaviour | Changed |
| `refactor`, `style` | Code changes with no visible effect | no |
| `docs` | Documentation only | Documentation |
| `test`, `ci`, `build`, `chore` | Tests, CI, build, housekeeping | no |

A `!` or a `BREAKING CHANGE:` footer marks a breaking change (for example a save format that old versions can't read).
Scopes are optional: `bots`, `battle`, `shop`, `ui`, `i18n`, `save`, `docs`, `ci`.

Enforced twice:

- locally: a prek `commit-msg` hook runs cocogitto's `cog verify` (cocogitto comes from `shell.nix`);
- on GitHub: `amannn/action-semantic-pull-request` checks PR titles, which become the commits on `main`.

## CI (every push and pull request)

- Linux: all prek hooks (formatting, clippy, ASCII, Markdown, whitespace), then `cargo test`.
- Release builds on Linux, macOS and Windows, so platform breakage shows up before release day.
- Rust comes from `mise.toml` via `jdx/mise-action`; dependencies are cached (`Swatinem/rust-cache`).
- Checkouts use `fetch-depth: 0` so `build.rs` sees the tags for the version string.

## Releases

Tool: [release-plz](https://release-plz.dev) (changelog by git-cliff), publishing to crates.io turned off.

1. Every merge to `main` updates a **release PR**: version bump in `Cargo.toml` / `Cargo.lock` (`fix` = patch, `feat`
   = minor, breaking = minor while below 1.0) and the new `CHANGELOG.md` section.
2. Review the PR, polish the wording if needed, merge it.
3. release-plz tags `vX.Y.Z` and creates the GitHub Release with that changelog section as notes.
4. The same workflow then builds the binaries on native runners and attaches them (`taiki-e/upload-rust-binary-action`).
   It must be the same workflow: tags pushed with `GITHUB_TOKEN` don't trigger other workflows.

Release files:

| Platform | File |
|---|---|
| Linux x86_64, ARM64 | `scorch-vX.Y.Z-linux-<arch>.tar.gz` and `Scorch-vX.Y.Z-<arch>.AppImage` |
| Windows x86_64, ARM64 | `scorch-vX.Y.Z-windows-<arch>.zip` (no console window) |
| macOS (Intel + Apple Silicon) | `scorch-vX.Y.Z-macos.zip` with a universal `Scorch.app` and icon |
| All | `SHA256SUMS`, and a signature per file for self-update |

- Linux builds run on an older Ubuntu for wider glibc compatibility; ARM64 on GitHub's ARM runners.
- Builds aren't code-signed at first: release notes explain SmartScreen ("More info" > "Run anyway") and macOS
  (right-click > Open). Signing (Apple Developer ID, Windows certificate) can come later.

## Self-update

- **Consent:** the first start asks once "Check for updates automatically?" (Yes / No). Saved as Settings > "Check for
  updates"; Settings also has "Check now".
- **Check:** in the background on start (if allowed), one request to the GitHub releases API. Nothing about the player
  is sent.
- **Offer:** the main menu shows "Version X is available" with **Update** and **Later**, and a link to the release
  notes.
- **Update:** download this platform's file, verify its signature, replace the program, restart into the new version.
  - Signatures: ed25519 (`zipsign`); the private key is a GitHub secret used by the release workflow, the public key is
    built into the game. A file that fails verification is never installed.
  - Replacement: the `self_update` crate (uses `self-replace`, which handles running executables on Windows). On macOS
    the binary inside `Scorch.app` is replaced; for an AppImage, the AppImage file itself (`$APPIMAGE`).
- **Off** for development builds (versions with `+g...`, `.dirty`, `+nogit`) and for builds made by package managers
  (a `self-update` Cargo feature, on by default, that packagers turn off).
- Cost: HTTPS (rustls) adds about 1-2 MB to the binary. Downloads run on a background thread so the game never stalls.

## Order of work

1. `git init`, first commit, tag `v0.1.0`, `make hooks-install`; push to GitHub as `scorch-game` with the settings
   above.
2. CI workflow, PR title check, Dependabot, issue templates, README badges.
3. Release workflow: release-plz + native builds + AppImage + macOS `.app`, first automated release.
4. Signing key and self-update (needs releases to exist to test against).
