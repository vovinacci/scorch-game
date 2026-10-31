# Scorch roadmap

What's done is described in `GUIDE.md`. This file lists what's next, roughly in order of fun per effort. Move items into
the guide as they ship.

## Next

1. **Parachutes and shields**, with the inventory screen growing an on/off switch per item (the reason the inventory is
   its own panel).
2. **Weapon strip** along the bottom of the battle screen: your weapons and ammo, the selected one highlighted.
3. **More weapons:** Roller, Napalm, Digger, Leapfrog, Funky Bomb, Tracer, Riot Bomb, Baby Nuke, Death's Head.
4. **Batteries** (restore health; matters with "Health limits power") and guidance items.
5. **Sound.**
6. **Tank movement** using fuel.
7. **More bot types** from the original: Poolshark (bounces shots off walls), Tosser, Chooser, Spoiler, Cyborg.

## Platforms and releases

- **GitHub, CI, releases and self-update:** planned in detail in [RELEASING.md](RELEASING.md) (Linux, Windows and macOS
  binaries, AppImage, generated changelog, signed self-update).
- **Browser build:** saves and stats in browser storage instead of files, crash reports to the browser console, replace
  `SystemTime` (panics in the browser), hide Quit, `make web`.

## Bigger ideas from the original

- More than 4 players (up to 10), teams.
- Everyone aims at once (simultaneous mode).
- Speech bubbles when tanks fire or die.
- Hostile environment: lightning and meteors.
- Economy options: interest on savings, changing prices, selling items back, scoring modes.
- Mouse aiming.
