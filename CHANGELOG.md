# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.0](https://github.com/vovinacci/scorch-game/compare/v0.8.0...v0.9.0) - 2026-11-14

### Documentation

- *(readme)* retake the screenshots for 0.9.0 ([#116](https://github.com/vovinacci/scorch-game/pull/116))
- *(balance)* re-measure bot types, difficulty and the ladder with items ([#115](https://github.com/vovinacci/scorch-game/pull/115))
- *(roadmap)* drop the items that have shipped ([#112](https://github.com/vovinacci/scorch-game/pull/112))

### Features

- *(shop)* give every weapon and item a small icon ([#118](https://github.com/vovinacci/scorch-game/pull/118))
- *(ladder)* even out the steps between the rungs ([#117](https://github.com/vovinacci/scorch-game/pull/117))
- *(items)* add heat, horizontal and vertical guidance ([#114](https://github.com/vovinacci/scorch-game/pull/114))
- *(items)* add batteries ([#113](https://github.com/vovinacci/scorch-game/pull/113))
- *(bots)* add the Chooser and re-measure difficulty and the ladder ([#111](https://github.com/vovinacci/scorch-game/pull/111))
- *(bots)* add the Tosser and the Poolshark ([#110](https://github.com/vovinacci/scorch-game/pull/110))
- *(bots)* add the Spoiler and the Cyborg ([#108](https://github.com/vovinacci/scorch-game/pull/108))

## [0.8.0](https://github.com/vovinacci/scorch-game/compare/v0.7.0...v0.8.0) - 2026-11-12

### Features

- *(ladder)* a real ladder: ten rungs, up on a win, down after two losses ([#101](https://github.com/vovinacci/scorch-game/pull/101))

### Fixes

- *(sound)* keep the sound device awake between sounds ([#107](https://github.com/vovinacci/scorch-game/pull/107))
- *(sound)* sounds start sooner on Linux: a smaller buffer in the audio library ([#105](https://github.com/vovinacci/scorch-game/pull/105))
- *(battle)* shells explode on the bottom of the screen ([#104](https://github.com/vovinacci/scorch-game/pull/104))
- *(items)* a Mag Deflector pushes harder and shows while it pushes its last shell ([#103](https://github.com/vovinacci/scorch-game/pull/103))
- *(ui)* the focus stays on its widget when others appear or disappear ([#102](https://github.com/vovinacci/scorch-game/pull/102))

## [0.7.0](https://github.com/vovinacci/scorch-game/compare/v0.6.0...v0.7.0) - 2026-11-11

### Changed

- *(terrain)* paint and upload only the columns of the ground that changed ([#87](https://github.com/vovinacci/scorch-game/pull/87))

### Documentation

- smaller README screenshots ([#88](https://github.com/vovinacci/scorch-game/pull/88))

### Features

- *(dev)* make balance prints firing order, digging out and deflector tables too ([#97](https://github.com/vovinacci/scorch-game/pull/97))
- *(weapons)* Leapfrog hits harder and costs less: 45 damage, radius 30, 2 for $80 ([#93](https://github.com/vovinacci/scorch-game/pull/93))
- *(battle)* tell a player whose barrel is under the dirt that the tank is buried ([#91](https://github.com/vovinacci/scorch-game/pull/91))
- *(shop)* the Last round panel lists every source of money with what it pays ([#89](https://github.com/vovinacci/scorch-game/pull/89))

### Fixes

- *(bots)* three bots spread their fire, so firing first is no handicap ([#99](https://github.com/vovinacci/scorch-game/pull/99))
- *(bots)* bots keep a pack of Riot Bombs to dig themselves out ([#98](https://github.com/vovinacci/scorch-game/pull/98))
- *(save)* keep a save file that can't be read, and tell the player ([#94](https://github.com/vovinacci/scorch-game/pull/94))
- *(items)* a Mag Deflector pays for every pass of a shell through its field ([#92](https://github.com/vovinacci/scorch-game/pull/92))
- *(dev)* --shot with a path that can't be saved says so instead of crashing ([#90](https://github.com/vovinacci/scorch-game/pull/90))
- *(weapons)* a buried tank's shot goes off once, where it is ([#83](https://github.com/vovinacci/scorch-game/pull/83))
- *(ui)* arrow keys follow rows and columns, panel by panel ([#84](https://github.com/vovinacci/scorch-game/pull/84))
- *(bots)* a buried bot digs itself out instead of firing its biggest weapon ([#85](https://github.com/vovinacci/scorch-game/pull/85))
- *(world)* thin wispy clouds instead of cartoon puffs ([#81](https://github.com/vovinacci/scorch-game/pull/81))

## [0.6.0](https://github.com/vovinacci/scorch-game/compare/v0.5.0...v0.6.0) - 2026-11-10

### Features

- *(weapons)* napalm burns as flickering flames and singes the ground ([#79](https://github.com/vovinacci/scorch-game/pull/79))
- *(terrain)* blasts leave burnt earth around craters that fades over turns ([#77](https://github.com/vovinacci/scorch-game/pull/77))
- *(menu)* New game warns that the saved game will be abandoned ([#76](https://github.com/vovinacci/scorch-game/pull/76))
- *(world)* sky scenery: stars, sun, clouds and far hills ([#75](https://github.com/vovinacci/scorch-game/pull/75))
- *(terrain)* paint the ground in layers with a palette per terrain type ([#74](https://github.com/vovinacci/scorch-game/pull/74))
- *(weapons)* a Tracer doesn't end your turn, one per turn, 2 for $50 ([#72](https://github.com/vovinacci/scorch-game/pull/72))
- *(setup)* coaching gives power advice for your next turn ([#71](https://github.com/vovinacci/scorch-game/pull/71))
- *(setup)* coaching shows on screen through your next turn ([#69](https://github.com/vovinacci/scorch-game/pull/69))

### Fixes

- *(ui)* arrow keys move to the nearest button that way, skipping greyed-out ones ([#80](https://github.com/vovinacci/scorch-game/pull/80))
- *(weapons)* a Roller fired by a buried tank explodes in the dirt ([#78](https://github.com/vovinacci/scorch-game/pull/78))
- *(ui)* text panels only on the day sky, and lighter ([#73](https://github.com/vovinacci/scorch-game/pull/73))
- *(ui)* battle log rows, readable text on the day sky ([#68](https://github.com/vovinacci/scorch-game/pull/68))

## [0.5.0](https://github.com/vovinacci/scorch-game/compare/v0.4.1...v0.5.0) - 2026-11-08

### Features

- *(setup)* aiming help: coaching after a miss, and a path preview ([#63](https://github.com/vovinacci/scorch-game/pull/63))
- *(setup)* Random wind, gravity, walls and sky, picked every round ([#59](https://github.com/vovinacci/scorch-game/pull/59))
- *(weapons)* Tracer paths stay on the field all round ([#60](https://github.com/vovinacci/scorch-game/pull/60))
- *(controls)* aim with the mouse ([#58](https://github.com/vovinacci/scorch-game/pull/58))
- *(weapons)* Tracer, Riot Bomb and Death's Head ([#56](https://github.com/vovinacci/scorch-game/pull/56))

### Fixes

- *(items)* a Mag Deflector's last energy still pushes the shell that used it ([#64](https://github.com/vovinacci/scorch-game/pull/64))
- *(bots)* rank weapons by the price of one shot; sharper balance table ([#62](https://github.com/vovinacci/scorch-game/pull/62))
- *(weapons)* balance prices from a measured table ([#61](https://github.com/vovinacci/scorch-game/pull/61))

## [0.4.1](https://github.com/vovinacci/scorch-game/compare/v0.4.0...v0.4.1) - 2026-11-08

### Documentation

- roadmap items move to GitHub issues; README shows the custom bot editor ([#53](https://github.com/vovinacci/scorch-game/pull/53))
- fresh README screenshots and features; --shot keeps only the game area ([#37](https://github.com/vovinacci/scorch-game/pull/37))

### Fixes

- *(ui)* settings over a paused battle leave out the HUD; controls list the weapon keys ([#40](https://github.com/vovinacci/scorch-game/pull/40))
- *(update)* fall back to the releases page redirect when the API fails ([#31](https://github.com/vovinacci/scorch-game/pull/31))

## [0.4.0](https://github.com/vovinacci/scorch-game/compare/v0.3.0...v0.4.0) - 2026-11-08

### Features

- Scorch icon on the window, Dock and taskbar, and in the Windows exe ([#29](https://github.com/vovinacci/scorch-game/pull/29))
- open settings from the pause menu ([#27](https://github.com/vovinacci/scorch-game/pull/27))
- 8-bit style sound effects, quieter in the background ([#28](https://github.com/vovinacci/scorch-game/pull/28))

## [0.3.0](https://github.com/vovinacci/scorch-game/compare/v0.2.1...v0.3.0) - 2026-11-05

### Features

- sound effects, synthesised in code, with a volume setting ([#26](https://github.com/vovinacci/scorch-game/pull/26))
- bots aim Rollers and Napalm at where they end up ([#25](https://github.com/vovinacci/scorch-game/pull/25))
- weapon strip in battle: see your weapons, pick one by key or click ([#24](https://github.com/vovinacci/scorch-game/pull/24))
- six new weapons: Baby Nuke, Roller, Leapfrog, Funky Bomb, Napalm, Digger ([#22](https://github.com/vovinacci/scorch-game/pull/22))

## [0.2.1](https://github.com/vovinacci/scorch-game/compare/v0.2.0...v0.2.1) - 2026-11-02

### Fixes

- keep the chosen difficulty when the ladder moves ([#19](https://github.com/vovinacci/scorch-game/pull/19))
- Mag Deflector spends energy on every shell it pushes ([#20](https://github.com/vovinacci/scorch-game/pull/20))

## [0.2.0](https://github.com/vovinacci/scorch-game/compare/v0.1.4...v0.2.0) - 2026-11-02

### Documentation

- plan a Windows installer and exe icon ([#12](https://github.com/vovinacci/scorch-game/pull/12))

### Features

- show shield energy above tanks and in the HUD ([#17](https://github.com/vovinacci/scorch-game/pull/17))
- turn order setting with fair defaults ([#15](https://github.com/vovinacci/scorch-game/pull/15))
- count sudden death in shots per tank ([#14](https://github.com/vovinacci/scorch-game/pull/14))
- parachutes and shields ([#13](https://github.com/vovinacci/scorch-game/pull/13))

## [0.1.3](https://github.com/vovinacci/scorch-game/compare/v0.1.2...v0.1.3) - 2026-11-01

### Documentation

- open the macOS app by clearing the quarantine mark ([#6](https://github.com/vovinacci/scorch-game/pull/6))

### Features

- update the game from GitHub Releases ([#8](https://github.com/vovinacci/scorch-game/pull/8))

## [0.1.2](https://github.com/vovinacci/scorch-game/compare/v0.1.1...v0.1.2) - 2026-11-01

### Fixes

- crash reports, macOS app signing and NixOS install docs ([#4](https://github.com/vovinacci/scorch-game/pull/4))

## [0.1.1](https://github.com/vovinacci/scorch-game/compare/v0.1.0...v0.1.1) - 2026-10-31

### Fixes

- *(ci)* pass the GitHub token to release-plz in GITHUB_TOKEN ([#2](https://github.com/vovinacci/scorch-game/pull/2))
