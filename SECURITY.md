# Security

Scorch is a single-player game, but it does a few things worth reporting problems with: it reads and writes save files,
and (planned) it downloads and installs its own updates, verifying a signature first.

If you find a way to make the game run code it shouldn't, install an unsigned update, or damage files outside its data
folder, please report it privately via
[GitHub's private vulnerability reporting](https://github.com/vovinacci/scorch-game/security/advisories/new) rather than
a public issue. Only the latest release is supported.
