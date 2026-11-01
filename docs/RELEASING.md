# Scorch: GitHub, CI and releases

Files: `.github/workflows/ci.yml`, `release.yml`, `pr-title.yml`, `.github/dependabot.yml`, `release-plz.toml`,
`tools/package.sh`, `packaging/` (desktop entry, macOS `Info.plist`), `assets/icon/scorch.svg` (app icon).

## GitHub repository

- Name `scorch-game` (the game and binary stay "Scorch" / `scorch`), public, MIT.
- Description: "Turn-based artillery game, a tribute to Scorched Earth (1991). Rust + macroquad. English and Ukrainian."
- Topics: `game`, `artillery`, `scorched-earth`, `turn-based`, `retro`, `rust`, `macroquad`, `gamedev`.
- Social preview: the battle screenshot (1280x640).
- README badges: CI, latest release, licence.
- Squash merges only; the squash commit takes the PR title and body, so the PR title becomes the commit.
- Rulesets: `main` needs a pull request with passing checks (Checks, the five builds, the Nix package, the PR title),
  linear history and signed commits, and can't be deleted or force-pushed; `v*` tags can't be moved or deleted.
  Repository admins can bypass both (for emergencies).
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

- locally: a prek `commit-msg` hook runs cocogitto's `cog verify` (`tools/verify_commit.sh`; cocogitto comes from
  `shell.nix`);
- on GitHub: `amannn/action-semantic-pull-request` checks PR titles, which become the commits on `main`.

## CI (every push and pull request)

- Linux: all prek hooks (formatting, clippy, ASCII, Markdown, whitespace), then `cargo test`.
- Release builds on Linux, macOS and Windows, so platform breakage shows up before release day.
- Rust comes from `mise.toml` via `jdx/mise-action`; dependencies are cached (`Swatinem/rust-cache`).
- Checkouts use `fetch-depth: 0` so `build.rs` sees the tags for the version string.

## Releases

Tool: [release-plz](https://release-plz.dev) (changelog by git-cliff), publishing to crates.io turned off.

1. Every merge to `main` updates a **release PR**: version bump in `Cargo.toml` / `Cargo.lock` (`fix` = patch, `feat` =
   minor, breaking = minor while below 1.0) and the new `CHANGELOG.md` section.
2. Review the PR, polish the wording if needed, merge it. Merge it last: it releases what is on `main` at that moment,
   and each feature or fix merged before it updates its version and changelog. Merging it early ships the pending
   changes with the version they had so far.
3. release-plz tags `vX.Y.Z` and creates the GitHub Release with that changelog section as notes.
4. The same workflow then builds the binaries on native runners, packs them with `tools/package.sh` and attaches them
   with `SHA256SUMS`. It must be the same workflow: tags pushed with `GITHUB_TOKEN` don't trigger other workflows.
5. To rebuild the binaries of an existing release: Actions > Release > Run workflow, with its tag.

Release PRs made with `GITHUB_TOKEN` don't start CI, so their required checks never run. Give the workflow a
fine-grained personal access token as the secret `RELEASE_TOKEN` (this repository only; Contents and Pull requests: read
and write), and the release PR is checked like any other.

Release files:

| Platform | File |
|---|---|
| Linux x86_64, ARM64 | `scorch-vX.Y.Z-linux-<arch>.tar.gz` and `Scorch-vX.Y.Z-<arch>.AppImage` |
| Windows x86_64, ARM64 | `scorch-vX.Y.Z-windows-<arch>.zip` (no console window) |
| macOS (Intel + Apple Silicon) | `scorch-vX.Y.Z-macos.zip` with a universal `Scorch.app` and icon |
| Linux, Windows (for self-update) | `scorch-vX.Y.Z-linux-<arch>`, `scorch-vX.Y.Z-windows-<arch>.exe` |
| All | `SHA256SUMS` and its minisign signature `SHA256SUMS.minisig` |

- Linux builds run on an older Ubuntu for wider glibc compatibility; ARM64 on GitHub's ARM runners.
- No code signing with a paid certificate. Windows shows SmartScreen ("More info" > "Run anyway"). The macOS app has an
  ad-hoc signature (Apple Silicon refuses unsigned apps as "damaged"), but Gatekeeper still rejects it, so players
  remove the quarantine mark with `xattr` (README). Opening with a double-click would need an Apple Developer ID and
  notarization ($99/year); not used.

## Supply chain

- Every GitHub Action is pinned to a full commit SHA, with the version in a comment (`# v7.0.1`); Dependabot updates
  both. Python tools are pinned with hashes in `requirements.txt`; appimagetool is a pinned release checked against its
  sha256. Runner images are pinned (`ubuntu-24.04`, `windows-2025`, `macos-15`), not `-latest`.
- Workflows start with no permissions and each job asks only for what it needs; checkouts don't keep credentials;
  release jobs don't use caches. The PR title check runs on `pull_request`, not `pull_request_target`.
- Dependabot waits 7 days after a release before proposing it (`cooldown`).
- Workflows are checked with [actionlint](https://github.com/rhysd/actionlint) and [zizmor](https://docs.zizmor.sh)
  (both in nixpkgs).

## Self-update

The game updates itself from GitHub Releases (`src/update.rs`).

- **Consent:** the main menu asks once "Check for new versions … automatically?"; the answer is Settings > "Check for
  updates". About has a "Check for updates" button.
- **Check:** in the background at start (if allowed), one request to the GitHub releases API. A newer version shows
  **Update** / **Later** on the main menu; a failed check stays silent there (About shows it).
- **Update:** downloads `SHA256SUMS` and `SHA256SUMS.minisig`, verifies the minisign signature with the public key built
  into the game (`assets/release-key.pub`), downloads this platform's file and checks its SHA-256. Only then it replaces
  the game and offers **Restart**.
  - Linux and Windows: the plain program (`scorch-vX.Y.Z-linux-<arch>`, `scorch-vX.Y.Z-windows-<arch>.exe`), swapped in
    with `self-replace` (handles the running executable on Windows).
  - AppImage: the new AppImage replaces the file in `$APPIMAGE`.
  - macOS: the zip is unpacked next to `Scorch.app` with `ditto`, then the bundles are swapped. A bundle the game
    downloaded itself carries no quarantine mark.
- **Signing:** the release workflow signs `SHA256SUMS` with the minisign secret key in the secret `RELEASE_SIGNING_KEY`
  (unencrypted); a release without it fails. Losing the key means players update by hand once to a release with a new
  public key.
- **Off** for development builds (versions with `+`) and without the `self-update` Cargo feature (on by default; the Nix
  flake builds without it). HTTPS is rustls with Mozilla's root certificates built in: no OpenSSL, no system certificate
  store.

## Commit messages from tools

Dependabot uses `build:` for Cargo and pip, `ci:` for Actions updates; the flake.lock workflow uses `build:`;
release-plz's release PR is `chore: release vX.Y.Z`. Commits and PRs carry no trailers (no "Co-authored-by" or
"Generated with" lines).
