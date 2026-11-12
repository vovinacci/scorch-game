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
| Left / Right | Turn the barrel | Change the selected setting, or move to the button on that side |
| Up / Down | Shot power | Move to the option above or below (greyed-out ones are skipped) |
| Shift (hold) | Fine adjustment (5 times slower) | |
| Tab | Next weapon you have ammo for | |
| 1-9, 0, -, = | Pick a weapon from the weapon strip (or click it) | |
| Space | Fire | Choose |
| Enter | Skip to the end of the round (only bots left) | Choose |
| L (hold) | Full battle log of this round | |
| Esc | Pause menu | Back |
| Mouse: drag (left button) | Aim: the barrel points at the cursor, power grows with the distance | |
| Mouse wheel | Power, finely (10 a notch, 1 with Shift) | |
| Mouse: right button | Fire | |

The mouse works everywhere: point at a button to select it, click to choose.

**Aiming with the mouse:** on your turn, press the left button anywhere on the battlefield and drag. The barrel points
from your tank toward the cursor, and the further away the cursor is, the more power (the full 1000 at about 500 pixels;
never more than your tank can give). A dotted line shows the aim, and the numbers at the top follow it. Hold Shift while
dragging to fine-tune: moves count 5 times less. Letting go keeps the aim and doesn't fire; fire with the right button
or Space. A press on the weapon strip picks a weapon instead of aiming.

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

Above the bottom line, the **weapon strip** lists every weapon you have ammo for, with how many are left. The selected
one has a gold frame. With more than 7 weapons it takes two rows. If a tank stands behind the strip (deep down in a
valley or a crater), the tank is drawn on top of the strip so you can still see it. On your turn, click a weapon or
press the key in front of it (the first 12 have keys; Tab reaches the rest).

While a bot aims, the bottom line tells you which tank it's aiming at and why. On your turn, the bottom line lists the
controls.

## How a round works

### Turns

Tanks fire one at a time, in a cycle that repeats until the round ends, when one tank or none is left. Their positions
on the hills are shuffled every round. Who starts and the order after that depend on the **Turn order** option:

- **Losers first** (the default): whoever is in last place starts (fewest round wins, then fewest kills); a tie is
  broken at random. The rest of the order is shuffled every round, so nobody always fires right after the same opponent.
  In round 1 everyone is tied, so it's random.
- **Winners first:** the same, but the leader starts.
- **Random:** a new random order every round.
- **Round robin:** always the same order (players first, then bots), with the first turn moving one place each round.

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
radius 35 and damage 40; each shell aboard adds to that, up to radius 110 and damage 120 (Baby Missiles, Dirt Bombs and
Diggers don't count). The inventory in the shop shows how big your own blast would be. Carrying a stack of Nukes into
battle is a risk to everyone around you, you included.

### Who gets the kill

A kill always goes to the last **enemy** who damaged the tank this round (or took the ground from under it with a Riot
Bomb or a Digger), however it finally died: by that enemy's shot, by a fall, by its own shot, by self-destructing or by
sudden death. So if you weaken a tank and it then blows itself up, the kill is yours. A tank that no enemy ever hit dies
without anyone getting a kill.

## Weapons

| Weapon | What it does | Radius | Damage | Price | You get |
|---|---|---|---|---|---|
| Baby Missile | Explodes | 20 | 30 | free | unlimited |
| Tracer | Harmless, and doesn't end your turn (one per turn): its path stays on the battlefield until the round ends, in your colour, for everyone to see | - | 0 | $50 | 2 |
| Missile | Explodes | 35 | 55 | $60 | 2 |
| MIRV | Splits into 5 warheads at the top of its arc | 25 each | 35 each | $120 | 1 |
| Nuke | A huge explosion | 85 | 100 | $200 | 1 |
| Dirt Bomb | Throws up a mound of earth, no damage | 45 | 0 | $40 | 2 |
| Digger | Burrows 160 pixels sideways under the ground, which sinks 40 pixels above it; no damage | - | 0 | $40 | 2 |
| Riot Bomb | Clears away the dirt where it lands; no blast damage, but a tank whose ground is cleared falls | 60 | 0 | $50 | 2 |
| Roller | Lands, rolls downhill and explodes in the next dip, at a tank or at a slope it can't climb | 30 | 50 | $120 | 2 |
| Leapfrog | Explodes, then hops on and explodes again, three blasts in all | 30 each | 45 each | $80 | 2 |
| Napalm | Spills burning fuel that runs downhill and burns for 3 seconds | - | 20 per second | $110 | 1 |
| Baby Nuke | A big explosion | 55 | 75 | $130 | 1 |
| Funky Bomb | Explodes and throws 6 bomblets around at random | 22 each | 28 each | $120 | 1 |
| Death's Head | Splits into 9 warheads at the top of its arc, spread wider than the MIRV's | 35 each | 45 each | $250 | 1 |

Everyone starts the game with $100 and 2 Missiles. Weapons and money carry over from round to round. The Dirt Bomb is
for burying an enemy or building a wall in front of yourself; the Digger drops an enemy into a trench (the fall hurts);
the Riot Bomb clears a mound away, digs out a buried tank or opens a path through a hill. A buried tank's shot goes off
once, in the dirt around it, and hurts the tank itself: a Roller can't roll out, a Leapfrog doesn't hop on, a Funky Bomb
throws no bomblets, and Napalm is smothered (no fire at all). So dig out with a Riot Bomb: one shot, no damage. A Digger
does no damage either, but it only clears the side it burrows towards, and a direct hit takes two of them. Without
either, the Baby Missile costs the least health. While your barrel is under the dirt, the lines under the turn counter
say "Buried: your shot goes off around you". A Tracer is a look before your real shot: fire it, see where that aim comes
down in this turn's wind, then adjust and fire for real (after a Tracer your weapon goes back to the one you fired
last). Only one per turn. Every Tracer path stays as a dashed line until the round ends, for everyone to see, and your
last real shot's path (with "Last shot path" on) stays as it was. At 25 dollars a look it pays off before a big shot,
not before every one. A Tracer doesn't count as a shot in your accuracy and doesn't drain a Mag Deflector. Rollers and
Napalm find the low ground, so they're best against a tank in a valley. Napalm burns any tank close to the fire, yours
too, and shields soak up the burn like any other damage.

## Parachutes and shields

Besides weapons, the shop sells items that protect your tank:

| Item | What it does | Price | You get |
|---|---|---|---|
| Parachute | Opens by itself when a fall would hurt; you drift down without damage. One per fall | $40 | 2 |
| Shield | Takes up to 50 damage before your health does | $120 | 1 |
| Heavy Shield | Takes up to 100 damage before your health does | $200 | 1 |
| Mag Deflector | Pushes enemy shells away as they come close; 40 energy, 10 each time it pushes a shell away | $150 | 1 |

- At the start of every round, your strongest shield goes up by itself (Heavy Shield, then Shield, then Mag Deflector)
  and lasts until its energy is used up or the round ends. It's used up either way, so a new round needs a new shield. A
  glowing ring around the tank shows it, fading as it weakens. Above the tank, a second bar in the shield's colour sits
  over the health bar and shrinks as the shield loses energy (with the health number on, "+40" next to it shows the
  energy left). During your turn, the top right of the screen shows your shield and its energy, for example "Heavy
  Shield: 60/100". Light blue is the Shield, dark blue the Heavy Shield, purple the Mag Deflector.
- The Mag Deflector bends shells away from your tank, sometimes enough to miss. Bots know this and aim around it, as you
  can. It doesn't touch your own shells. Each time an enemy shell comes into its field costs it 10 energy: every MIRV
  warhead counts, and so does a shell that is pushed out and falls back in. So it pushes shells away about four times,
  fewer if blasts land near you. The shell that takes its last energy is still pushed until it leaves the field; if it
  comes back, nothing stops it. Once that turn's shells are done, the deflector goes down like any shield.
- Each item has an on/off switch in the inventory. Switched off, it stays in your inventory unused, for example to save
  your only Heavy Shield for later rounds.
- Shields stop every kind of damage from blasts, including death blasts and self-destructs, but not falls (that's what
  parachutes are for) or sudden death.

## Money and the shop

You earn money every round:

| For | Amount |
|---|---|
| Damage to other tanks | $1 per point of damage, shields included |
| Each kill | $50 |
| Bounty | $50 extra per round the destroyed tank has won |
| Winning the round | $100 |
| Taking part (everyone) | $75 |
| Loss bonus | $40 per round lost in a row, up to 4 ($160); a win resets it |

The game helps whoever is behind, so one bad start doesn't decide the whole game:

- everyone gets round pay, win or lose,
- the loss bonus grows with every loss in a row,
- the bounty makes the leader a valuable target,
- the player in last place shoots first next round (with the default turn order).

Between rounds each human player gets a screen with three panels: what you earned last round (left: every source of
money with what it pays, earned or not), the **Shop** (middle: weapons and items) and your **Inventory** (right: what
you own, a switch for each item, and your death blast). In a two-player game the shop title shows whose turn it is to
buy. Bots shop too: first a pack of Riot Bombs if they have none (their way out from under a Dirt Bomb), then random
weapons (never Dirt Bombs or Diggers) and items they can afford, and they use them like you do.

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

Bots follow "Health limits power" too. They usually fire their most expensive weapon (by the price of one shot),
otherwise a Baby Missile. They never fire Dirt Bombs or Diggers, and aim Rollers and Napalm at the spot where they roll
or run to. A buried bot digs itself out: with a Riot Bomb if it has one, otherwise with a Missile if one shot gets it
out and it has the health to spare, otherwise with Baby Missiles, a little health and a turn at a time.

### Whom a bot shoots at

1. **Revenge:** whoever hit it since its last shot.
2. Otherwise the same target as last time, while it's alive (it keeps the aim it has already learned).
3. Otherwise it picks a new target, preferring tanks that are **nearest**, **weakest** or the **leader** (most round
   wins). It avoids a tank that other bots are already aiming at, so bots don't all gang up on you.

When only three bots are left (no human), they spread their fire so that each has one other after it: a bot rather
doesn't pick the one that is aiming at it, and it leaves revenge to the third bot if that one is already after the
attacker. Otherwise two bots would end up on whichever fired first.

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

### The ladder

Switch **Ladder** on (one player only) to climb ten rungs of opponents. The rung sets the bots you face, so the Bots and
Difficulty options don't apply, and the setup screen shows your rung, whom you are up against and your best rung so far.

| Rung | Opponents |
|---|---|
| 1 | Moron |
| 2 | Moron, Moron |
| 3 | Shooter |
| 4 | Shooter, Shooter |
| 5 | Sniper, Shooter |
| 6 | Shooter, Shooter, Shooter |
| 7 | Sniper, Shooter, Moron |
| 8 | Sniper, Shooter, Shooter |
| 9 | Sniper, Sniper, Shooter |
| 10 | Sniper, Sniper, Sniper |

- **Win the game outright** (more round wins than any bot) and you go up a rung.
- **Lose twice in a row** on a rung and you go down one. A win in between wipes the first loss out. There is no way down
  from rung 1.
- A **shared first place** changes nothing.
- **Abandoning** a ladder game counts as a loss.
- Win on rung 10 and you are **champion**; you stay on rung 10 and can do it again.
- A game played with the **Short path** or **Full path** aiming help doesn't count, either way. Coaching is fine.

The final screen says what the game did to your rung, and **Statistics** keeps your best rung and how often you were
champion. Your place on the ladder stays when you switch the ladder off and play ordinary games in between.

## Game options

Choose these under **New game**. Your choices are remembered for next time. **I'm feeling lucky** sets everything at
random except the number of human players; you can still check and change it all before starting.

### Match

| Option | Choices | Default |
|---|---|---|
| Players | 1, 2 (hotseat: take turns at one computer) | 1 |
| Bots | 1 to 3 (4 tanks at most) | 2 |
| Difficulty | Easy, Normal, Hard, Brutal, Custom | Normal |
| Ladder | Off, On (see The ladder) | Off |
| Rounds | 3, 5, 7, 10 | 5 |
| Health limits power | Off, On | On |
| Sudden death | Off, 8/12/16/24 shots each | 12 shots each |
| Turn order | Losers first, Winners first, Random, Round robin | Losers first |

### World

| Option | Choices | Default |
|---|---|---|
| Terrain | Rolling hills, Mountains, Flatlands, Canyon, Random | Rolling hills |
| Wind | Off, Calm (up to 20), Normal (up to 40), Strong (up to 70), Random | Normal |
| Wind changes each turn | Off, On | On |
| Gravity | Low, Normal, High, Random | Normal |
| Walls | Open, Solid, Bouncy, Wrap-around, Random | Open |
| Sky | Night, Sunset, Day, Random | Night |
| Aiming help | Off, Coaching, Short path, Full path | Off |

**Random** picks a new value at the start of every round. During the round, the lines under the turn counter say what
gravity and walls came out as (you can see the terrain and sky, and the wind is in the HUD), the pause menu's "This
game" shows every pick in blue, and the battle log starts the round with the full list. "I'm feeling lucky" sets real
values, never Random.

**Aiming help** is for learning, and it's the same for every player in the game (bots don't need it):

- **Coaching:** after a shot that hits nobody, your next turn starts with advice for the enemy you were aiming at, for
  your current angle and that turn's wind: "a little more power", "more power", "a lot more power" (or less), "about the
  right power; check your angle", "out of reach at this angle; aim higher" (or lower), or that the enemy is out of reach
  from where you stand (your damage may be limiting your power). It never gives the exact numbers. It stays just above
  the weapon strip until you fire, and goes into the full battle log.
- **Short path:** coaching, plus a dotted line showing the first third of where your shot will fly, wind included, while
  you aim.
- **Full path:** coaching, plus the whole path to where the shot comes down. Aiming becomes easy: good for learning or a
  relaxed game.

The pause menu's "This game" shows the setting, so everyone knows what help was on.

Walls decide what happens at the edges of the screen:

- **Open:** shells that fly off the side are lost.
- **Solid:** shells explode against the wall.
- **Bouncy:** shells bounce back, a bit slower.
- **Wrap-around:** a shell leaving one side comes back in on the other.

A small preview under the World options shows the terrain and sky.

Each terrain has its own look: rolling hills are green over brown earth, mountains are grey rock with snow on the
highest peaks, flatlands are grassy, and the canyon is red sandstone in layers. The ground gets darker the deeper it
goes, so craters show what's underneath. Explosions leave burnt, darker earth around their craters, and napalm singes
the ground where it burns; it fades over a few turns, and then the grass grows back.

Each sky has its own scenery: stars at night, a big low sun at sunset, the sun and a few clouds by day, and faint far
hills on the horizon. It's only a backdrop: shots fly straight through it.

## When a round drags on

- **Only bots left:** once no human tank is in the round, it plays at the bots-only speed (4x by default) and shows
  **Skip to end [Enter]**, which finishes the round instantly.
- **Sudden death** (option): once every tank has had the chosen number of shots (8, 12, 16 or 24 each, so the fight
  lasts as long with 2 tanks as with 4), every tank loses 5 health at the start of each turn until the round ends.
  Shields don't stop it. The top right of the screen shows the turn it starts.
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

All text over the battlefield has a thin dark outline, so it's readable over any sky or ground. On the bright day sky,
the lines at the top (the battle log on the left, the turn and shield on the right) and the coaching line also sit on a
light panel. The dots of the aiming lines have a dark edge for the same reason.

## Menus

- **Main menu:** Continue (when you have a saved game), New game, Statistics, Settings, About, Quit.
- **Pause menu** (Esc during battle): Resume, Flee this round, Self-destruct, Battle log, Settings, Save & exit to menu,
  Save & quit game, Abandon game. Flee and Self-destruct are greyed out unless you're aiming. Next to it, "This game"
  shows the round, the turn and all options of the current game. Settings opens the same settings as the main menu; Back
  returns to the paused battle.
- **Settings:**
  - Display: **Language** (English, Українська), tank health (Bar, Bar + number, Number), last shot path, battle log,
    fullscreen.
  - Gameplay: game speed (Normal, Fast 1.5x, Faster 2x), bots-only speed (2x, 4x, 8x, on top of the game speed),
    **Sound** (Off, Low, Medium, High; Medium at first) and **Check for updates** (see below). Sound is in the style of
    old 8-bit consoles and stays in the background; it covers shots, explosions, shield hits, parachutes, fire, digging,
    menu clicks and the end of a round. If your computer has no working sound device, the game simply stays silent.
  - A list of all controls.
- **About:** the dedication, the author, the original game and the credits, the game's version and a
  **Check for updates** button.
- **Final standings:** after the last round, with Play again (same options) and Main menu.

## Updates

Scorch can update itself. On the first start the main menu asks once whether to check for new versions automatically;
your answer is the **Check for updates** setting. When a new version is out, the main menu says so: **Update** downloads
and installs it, then **Restart** starts the new version; **Later** hides the notice until the next start. You can also
check by hand on the About screen.

Checking sends one request to GitHub, where Scorch is published, and nothing about you. Every update is signed by the
author: if a download doesn't match the signature, it isn't installed. Versions installed with a package manager (such
as the Nix package) are updated by that package manager instead and don't show these options.

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
  saved also abandons it: New game asks first.
- Your saved game, settings and statistics are kept in your user data folder, for example `~/.local/share/scorch/` on
  Linux.
- If one of those files can't be read (it is damaged, or was saved by a newer version of Scorch), the game starts afresh
  without it, keeps the old file next to it as `profile.unreadable.json` or `run.unreadable.json`, and says so on the
  main menu.

## Statistics

**Statistics** on the main menu shows your all-time record (human players only): games played, won and abandoned, most
rounds won in a game, rounds played and won, kills, deaths, rounds fled, shots fired, damage dealt, damage per shot,
your biggest single hit and money earned. You win a game by having the most round wins at the end (a shared first place
counts). Abandoned games don't count as played, but their finished rounds do.

## If the game crashes

The game writes a crash report to your user data folder. On the next start the main menu tells you and shows where the
report is. If it crashed several times since you last looked, one notice covers all of them. **Copy report** puts the
reports on the clipboard so you can paste them into a message to the author; **Dismiss** hides the notice. Either way,
those reports aren't offered again. Nothing is ever sent anywhere by itself.

The version number is shown at the bottom right of the main menu and on the About screen; please include it when you
report a problem.

## Tips

- Your first shot at a new target rarely hits. Use it to measure, then adjust in small steps with Shift held.
- High arcs are at the wind's mercy; flat shots are more predictable.
- Don't sit next to a tank that's about to die, especially one carrying Nukes.
- A Dirt Bomb on a tank in a valley can bury it.
- Falling hurts: digging a crater under an enemy on a cliff can do more than a direct hit.
- When you're behind, remember the bounty: destroying the leader pays extra.
