# Scorch: player's guide

Scorch is a turn-based artillery game and a tribute to *Scorched Earth* (1991) by Wendell Hicks. Tanks stand on hills
that can be blown apart. They take turns firing shells that fly under gravity and drift with the wind. The last tank
standing wins the round. Between rounds you spend your money in a shop, and after the last round the player with the
most round wins takes the game.

The game is dedicated to the memory of Joseph Rivkind (1952–2026), who taught its author to never stop learning.

## Your first game

1. Choose **New game**, keep the suggested settings and press **Start game**.
2. Your tank is marked "You". When it's your turn, a yellow marker appears above it.
3. Turn the barrel with **Left / Right**, set the power with **Up / Down**, and press **Space** to fire.
4. Watch where the shell lands and correct the next shot. The faint dotted line shows your previous shot.
5. Destroy the other tanks before they destroy you. Between rounds, buy better weapons in the shop.

## Controls

| Key | In battle | In menus |
|---|---|---|
| Left / Right | Turn the barrel | Change the selected setting |
| Up / Down | Shot power | Move between options |
| Shift (hold) | Fine adjustment (5 times slower) | |
| Tab | Next weapon you have ammo for | |
| Space | Fire | Choose |
| Enter | Skip to the end of the round (only bots left) | Choose |
| L (hold) | Full battle log of this round | |
| Esc | Pause menu | Back |

The mouse works everywhere: point at a button to select it, click to choose.

## The battle screen

The bar at the top shows, from left to right:

- whose turn it is,
- **Angle** (0-180 degrees; 90 is straight up),
- **Power** (0-1000; shown as `current/max` when health limits power),
- the selected weapon and how many you have (∞ for the Baby Missile),
- your money and what you've earned this round,
- **WIND**: the arrow shows its direction and the number its strength ("off" when the world has no wind),
- the round number.

Under the bar, on the right, is the turn number (each tank's shot is one turn), plus when sudden death starts, if it's
on.

Each tank has its name and a health bar above it. In Settings you can show health as a bar, a number, or both.

While a bot aims, the bottom line tells you which tank it's aiming at and why. On your turn, the bottom line lists the
controls.

## How a round works

### Turns

Tanks fire one at a time, in order. Their positions are shuffled every round. Round 1 starts with a random tank; later
rounds start with whoever is in last place (fewest round wins, then fewest kills). The round ends when one tank or none
is left.

### Aiming and the wind

The shell flies in an arc. Wind pushes it sideways the whole way, so a long, high shot drifts much more than a short,
flat one. Wind starts at a random strength and, if "Wind changes each turn" is on, shifts a little every turn. Check it
before each shot.

### Health and power

Every tank starts each round with 100 health. With **Health limits power** on (the default, as in the original game),
your maximum power is your health x 10: at 30 health you can shoot at most 300, which makes distant targets hard to
reach. The power never drops below 50, so even a badly damaged tank can lob a short shot.

### Damage

- An explosion damages every tank near it: full damage at the centre, down to 30% at the edge. Your own explosions hurt
  you too, so don't fire at something right next to you.
- Explosions dig craters. When the ground under a tank is blown away, the tank falls. A short drop is harmless; a long
  fall costs health.
- **Wrecks** of destroyed tanks stay on the ground.
- If every tank is destroyed, the round is a draw and nobody wins it.

### When a tank is destroyed

A tank at 0 health **explodes**. That blast can destroy its neighbours, which explode in turn: chain reactions are part
of the game. The ammo a tank carries cooks off and makes its blast bigger: a tank with an empty hold explodes with
radius 35 and damage 40; each shell aboard adds to that, up to radius 110 and damage 120 (Baby Missiles and Dirt Bombs
don't count). The inventory in the shop shows how big your own blast would be. Carrying a stack of Nukes into battle is
a risk to everyone around you, you included.

### Who gets the kill

A kill always goes to the last **enemy** who damaged the tank this round, however it finally died: by that enemy's shot,
by a fall, by its own shot, by self-destructing or by sudden death. So if you weaken a tank and it then blows itself up,
the kill is yours. A tank that no enemy ever hit dies without anyone getting a kill.

## Weapons

| Weapon | What it does | Radius | Damage | Price | You get |
|---|---|---|---|---|---|
| Baby Missile | Explodes | 20 | 30 | free | unlimited |
| Missile | Explodes | 35 | 55 | $60 | 3 |
| MIRV | Splits into 5 warheads at the top of its arc | 25 each | 35 each | $120 | 1 |
| Nuke | A huge explosion | 85 | 100 | $200 | 1 |
| Dirt Bomb | Throws up a mound of earth, no damage | 45 | 0 | $40 | 2 |

Everyone starts the game with $100 and 2 Missiles. Weapons and money carry over from round to round. The Dirt Bomb is
for burying an enemy or building a wall in front of yourself.

## Money and the shop

You earn money every round:

| For | Amount |
|---|---|
| Damage to other tanks | $1 per point of damage |
| Each kill | $50 |
| Bounty | $50 extra per round the destroyed tank has won |
| Winning the round | $100 |
| Taking part (everyone) | $75 |
| Loss bonus | $40 per round lost in a row, up to 4 ($160); a win resets it |

The game helps whoever is behind, so one bad start doesn't decide the whole game:

- everyone gets round pay, win or lose,
- the loss bonus grows with every loss in a row,
- the bounty makes the leader a valuable target,
- the player in last place shoots first next round.

Between rounds each human player gets a screen with three panels: what you earned last round and why (left), the
**Shop** (middle) and your **Inventory** (right). In a two-player game the shop title shows whose turn it is to buy.
Bots shop too: they buy random weapons they can afford.

## Opponents

### Bot skills

Bots come in three skills. Each one aims well, but misses on purpose at first and then corrects itself shot by shot, the
way a person would:

| Skill | First miss up to | Corrects each shot by | Typically on target by |
|---|---|---|---|
| Moron | 25° / 300 power | keeps 80% of its error | shot 11 |
| Shooter | 12° / 180 power | keeps 50% of its error | shot 4 |
| Sniper | 6° / 90 power | keeps 30% of its error | shot 2 |

How it works: a bot works out the perfect shot from the real wind, gravity and hills, then adds an error. Its first shot
at a new target misses by 40-100% of its "first miss" in a random direction. After each shot the error shrinks to the
kept share, plus a little random wobble that never quite goes away. A bot that switches to a new target starts with a
rough shot again.

Bots follow "Health limits power" too. They usually fire their biggest weapon, otherwise a Baby Missile.

### Whom a bot shoots at

1. **Revenge:** whoever hit it since its last shot.
2. Otherwise the same target as last time, while it's alive (it keeps the aim it has already learned).
3. Otherwise it picks a new target, preferring tanks that are **nearest**, **weakest** or the **leader** (most round
   wins). It avoids a tank that other bots are already aiming at, so bots don't all gang up on you.

The bottom line of the screen shows the target and the reason while a bot aims, and the full battle log keeps every
choice. Right after continuing a saved game, the reason isn't shown until the bot picks a new target.

### Difficulty

Difficulty sets a budget of "threat points" for the bots: a Moron costs 1, a Shooter 2, a Sniper 3. Every bot starts as
a Moron and random bots are upgraded until the budget is spent, so the same difficulty gives a different mix each game.
More bots get a little extra each, so more opponents is harder, but not overwhelming.

| Difficulty | vs 1 bot | vs 2 bots | vs 3 bots |
|---|---|---|---|
| Easy | 1 | 2 | 3 |
| Normal | 2 | 3 | 4 |
| Hard | 3 | 4 | 6 |
| Brutal | 3 | 6 | 9 |

**Custom** gives every bot your own profile. In **Edit custom bot** you set the first miss and how fast it learns, and
the screen shows how close its shots will typically be, shot by shot.

**Ladder:** when on, the difficulty goes up one step after you win a game and down one step after you lose, between Easy
and Brutal (Custom never changes). The final screen tells you when it changed.

## Game options

Choose these under **New game**. Your choices are remembered for next time. **I'm feeling lucky** sets everything at
random except the number of human players; you can still check and change it all before starting.

### Match

| Option | Choices | Default |
|---|---|---|
| Players | 1, 2 (hotseat: take turns at one computer) | 1 |
| Bots | 1 to 3 (4 tanks at most) | 2 |
| Difficulty | Easy, Normal, Hard, Brutal, Custom | Normal |
| Ladder | Off, On | Off |
| Rounds | 3, 5, 7, 10 | 5 |
| Health limits power | Off, On | On |
| Sudden death | Off, After 15/25/40 turns | After 25 turns |

### World

| Option | Choices | Default |
|---|---|---|
| Terrain | Rolling hills, Mountains, Flatlands, Canyon, Random | Rolling hills |
| Wind | Off, Calm (up to 20), Normal (up to 40), Strong (up to 70) | Normal |
| Wind changes each turn | Off, On | On |
| Gravity | Low, Normal, High | Normal |
| Walls | Open, Solid, Bouncy, Wrap-around | Open |
| Sky | Night, Sunset, Day | Night |

Walls decide what happens at the edges of the screen:

- **Open:** shells that fly off the side are lost.
- **Solid:** shells explode against the wall.
- **Bouncy:** shells bounce back, a bit slower.
- **Wrap-around:** a shell leaving one side comes back in on the other.

A small preview under the World options shows the terrain and sky.

## When a round drags on

- **Only bots left:** once no human tank is in the round, it plays at the bots-only speed (4x by default) and shows
  **Skip to end [Enter]**, which finishes the round instantly.
- **Sudden death** (option): from the chosen turn on, every tank loses 5 health at the start of each turn until the
  round ends.
- **Flee this round** (pause menu, while you're aiming): your tank leaves the round. You lose the round, but nobody gets
  a kill. If no human is left, the round ends at once and the healthiest tank wins it.
- **Self-destruct** (pause menu, while you're aiming): your tank explodes with a big blast (radius 75, damage 80),
  followed by its usual death blast. Enemies it destroys count as your kills.

## Battle log

- **Feed** (top left): the last few events, fading after 8 seconds: hits with their damage, kills, fleeing,
  self-destructs and the start of sudden death. You can turn it off in Settings ("Battle log").
- **Full log:** hold **L**, or choose **Battle log** in the pause menu. It lists everything in this round by turn: every
  shot (weapon, angle, power, and "miss" if it hit nobody), every hit and kill, and each bot's choice of target with the
  reason. It starts empty each round.

All text over the battlefield has a thin dark outline, so it's readable over any sky or ground.

## Menus

- **Main menu:** Continue (when you have a saved game), New game, Statistics, Settings, About, Quit.
- **Pause menu** (Esc during battle): Resume, Flee this round, Self-destruct, Battle log, Save & exit to menu, Save &
  quit game, Abandon game. Flee and Self-destruct are greyed out unless you're aiming. Next to it, "This game" shows the
  round, the turn and all options of the current game.
- **Settings:**
  - Display: **Language** (English, Українська), tank health (Bar, Bar + number, Number), last shot path, battle log,
    fullscreen.
  - Gameplay: game speed (Normal, Fast 1.5x, Faster 2x) and bots-only speed (2x, 4x, 8x, on top of the game speed).
  - A list of all controls.
- **About:** the dedication, the author, the original game and the credits, and the game's version.
- **Final standings:** after the last round, with Play again (same options) and Main menu.

## Languages

Scorch speaks English (British spelling) and Ukrainian. On the very first start it picks Ukrainian if your computer is
set to Ukrainian, otherwise English. You can change it at any time in Settings; it takes effect at once.

The language option is always the first row in Settings, marked with a globe and labelled "Language / Мова", and each
language is written in its own language. So if you ever switch to a language you can't read, you can still find your way
back.

A saved game continues fine in either language.

## Saving and continuing

- The game saves by itself at the start of every turn and when the shop opens. You can simply close the window and come
  back later.
- **Continue** on the main menu picks up where you left off. If you closed the game mid-shot, it continues from the
  start of that turn.
- **Abandon game** (pause menu or shop, after a confirmation) ends the game for good. Starting a new game while one is
  saved also abandons it.
- Your saved game, settings and statistics are kept in your user data folder, for example `~/.local/share/scorch/` on
  Linux.

## Statistics

**Statistics** on the main menu shows your all-time record (human players only): games played, won and abandoned, most
rounds won in a game, rounds played and won, kills, deaths, rounds fled, shots fired, damage dealt, damage per shot,
your biggest single hit and money earned. You win a game by having the most round wins at the end (a shared first place
counts). Abandoned games don't count as played, but their finished rounds do.

## If the game crashes

The game writes a crash report to your user data folder. On the next start the main menu tells you and shows where the
report is. **Copy report** puts it on the clipboard so you can paste it into a message to the author; **Dismiss** hides
the notice. Nothing is ever sent anywhere by itself.

The version number is shown at the bottom right of the main menu and on the About screen; please include it when you
report a problem.

## Tips

- Your first shot at a new target rarely hits. Use it to measure, then adjust in small steps with Shift held.
- High arcs are at the wind's mercy; flat shots are more predictable.
- Don't sit next to a tank that's about to die, especially one carrying Nukes.
- A Dirt Bomb on a tank in a valley can bury it.
- Falling hurts: digging a crater under an enemy on a cliff can do more than a direct hit.
- When you're behind, remember the bounty: destroying the leader pays extra.
