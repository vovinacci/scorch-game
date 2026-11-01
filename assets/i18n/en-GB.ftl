## Scorch: English (GB). Keys must match uk.ftl (a test checks).
## Arguments named `you` / `target_you` are "yes" when that player is the
## solo human ("You"), for grammar.

## Common

back = Back
done = Done
cancel = Cancel
on = On
off = Off
save-exit-menu = Save & exit to menu
abandon-game = Abandon game
abandon = Abandon
unit-wins = { $n } { $n ->
    [one] win
   *[other] wins
}
unit-kills = { $n } { $n ->
    [one] kill
   *[other] kills
}

## Players

player-you = You
player-n = Player { $n }
bot-alpha = Alpha
bot-bravo = Bravo
bot-charlie = Charlie
skill-moron = Moron
skill-shooter = Shooter
skill-sniper = Sniper
skill-custom = Custom

## Intro and About

dedication-1 = In memory of Joseph Rivkind
dedication-years = 1952–2026
dedication-2 = who taught me to never stop learning
about-title = ABOUT
about-made-with = Made with ♥ by Volodymyr Shcherbinin (vovin) in Rust, using macroquad.
about-font = Font: Scorch Mono, based on Terminus by Dimitar Zhekov, SIL Open Font License.
about-version = Version

## Main menu

tagline = tanks, hills, and far too many explosives
menu-continue = Continue
menu-new-game = New game
menu-statistics = Statistics
menu-settings = Settings
menu-about = About
menu-quit = Quit
tribute = A tribute to Scorched Earth (1991) by Wendell Hicks, "The Mother of All Games"
crash-last-time = { $n ->
    [1] Scorch crashed last time. Please send the report.
   *[other] Scorch crashed { $n } times. Please send the reports.
}
crash-copy = { $n ->
    [1] Copy report
   *[other] Copy reports
}
crash-dismiss = Dismiss
crash-copied = { $n ->
    [1] Crash report copied to the clipboard. Paste it into a message.
   *[other] { $n } crash reports copied to the clipboard. Paste them into a message.
}

## New game

setup-title = NEW GAME
setup-match = Match
setup-world = World
setup-players = Players
setup-hotseat = 2 (hotseat)
setup-bots = Bots
setup-difficulty = Difficulty
setup-edit-custom = Edit custom bot
setup-ladder = Ladder
setup-rounds = Rounds
setup-power-limit = Health limits power
setup-sudden-death = Sudden death
setup-terrain = Terrain
setup-wind = Wind
setup-wind-changes = Wind changes each turn
setup-gravity = Gravity
setup-walls = Walls
setup-sky = Sky
setup-lucky = I'm feeling lucky
setup-start = Start game
setup-preview-walls = walls: { $walls }
difficulty-easy = Easy
difficulty-normal = Normal
difficulty-hard = Hard
difficulty-brutal = Brutal
difficulty-custom = Custom
difficulty-points = { $points } vs { $bots }
difficulty-note = threat points: { $points }   (Moron 1, Shooter 2, Sniper 3; mix is random)
difficulty-custom-note = every bot: { $summary }
bot-summary = first miss up to { $angle } deg / { $power } power, keeps { $keep }% of error per shot, { $hit }
bot-hits-by = usually on target by shot { $n }
bot-rarely-hits = rarely hits

SuddenDeath-Off = Off
SuddenDeath-After8 = 8 shots each
SuddenDeath-After12 = 12 shots each
SuddenDeath-After16 = 16 shots each
SuddenDeath-After24 = 24 shots each
TerrainKind-Rolling = Rolling hills
TerrainKind-Mountains = Mountains
TerrainKind-Flat = Flatlands
TerrainKind-Canyon = Canyon
TerrainKind-Random = Random
WindLevel-Off = Off
WindLevel-Calm = Calm
WindLevel-Normal = Normal
WindLevel-Strong = Strong
GravityLevel-Low = Low
GravityLevel-Normal = Normal
GravityLevel-High = High
Walls-Open = Open
Walls-Solid = Solid
Walls-Bouncy = Bouncy
Walls-Wrap = Wrap-around
Sky-Night = Night
Sky-Sunset = Sunset
Sky-Day = Day

## Game summary (pause menu)

summary-players = { $humans } vs { $bots } { $bots ->
    [one] bot
   *[other] bots
}
summary-round = Round
summary-round-of = { $round } of { $rounds }
summary-turn = Turn
summary-wind-changes = { $wind }, changes

## Custom bot

custom-title = CUSTOM BOT
custom-angle-error = First-shot angle error E (deg)
custom-power-error = First-shot power error E (power)
custom-learning = Learning L (error kept per shot)
custom-start-from = Start from:
custom-explanation = How a bot aims:
    1. It simulates about 2000 shots against the real wind and hills and finds the perfect one.
    2. On the first shot at a new target it misses on purpose: error = random 40% to 100% of E, in either direction.
    3. After every shot: next error = error x L + wobble. Wobble is random within +-12% of E.
    So the typical miss on shot n is 0.7 x E x L^(n-1).
    L = 0.5 halves the miss every turn. L = 0.9 barely learns. L = 0.1 is deadly from shot 2.
    Wobble never fully goes away (long-run wobble, right).
custom-typical-miss = Typical miss per shot at one target
custom-shot = Shot { $n }
custom-miss = { $angle } deg  { $power } pw
custom-wobble = Long-run wobble: +-{ $angle } deg, +-{ $power } power
custom-verdict-first = Hits on the first shot. Brutal.
custom-verdict-by = Usually on target by shot { $n }.
custom-verdict-never = Rarely gets close. Easy prey.
custom-on-target = (on target = under { $angle } deg and { $power } power off)

## Settings

settings-title = SETTINGS
settings-display = Display
settings-gameplay = Gameplay
settings-controls = Controls
settings-language = Language / Мова
settings-health = Tank health
settings-shot-path = Last shot path
settings-battle-log = Battle log
settings-fullscreen = Fullscreen
settings-game-speed = Game speed
settings-bot-speed = Bots-only speed
Language-English = English
Language-Ukrainian = Українська
HealthDisplay-Bar = Bar
HealthDisplay-BarAndNumber = Bar + number
HealthDisplay-Number = Number
GameSpeed-Normal = Normal
GameSpeed-Fast = Fast (1.5x)
GameSpeed-Faster = Faster (2x)
BotSpeed-X2 = 2x
BotSpeed-X4 = 4x
BotSpeed-X8 = 8x
controls-aim-key = Left / Right
controls-aim = Turn the barrel
controls-power-key = Up / Down
controls-power = Shot power
controls-fine-key = Shift (hold)
controls-fine = Fine adjustment
controls-weapon-key = Tab
controls-weapon = Next weapon
controls-fire-key = Space
controls-fire = Fire
controls-log-key = L (hold)
controls-log = Full battle log
controls-pause-key = Esc
controls-pause = Pause menu / back
controls-skip-key = Enter
controls-skip = Skip rest of round (bots only)
controls-menu-move-key = Menus: Up / Down
controls-menu-move = Move between options
controls-menu-change-key = Menus: Left / Right
controls-menu-change = Change a setting
controls-menu-choose-key = Menus: Enter
controls-menu-choose = Choose

## Statistics

stats-title = ALL-TIME STATS
stats-games-played = Games played
stats-games-won = Games won
stats-games-abandoned = Games abandoned
stats-best-game = Most rounds won in a game
stats-rounds-played = Rounds played
stats-rounds-won = Rounds won
stats-kills = Kills
stats-deaths = Deaths
stats-rounds-fled = Rounds fled
stats-shots = Shots fired
stats-damage = Damage dealt
stats-damage-per-shot = Damage per shot
stats-best-hit = Biggest single hit
stats-money = Money earned

## Battle

hud-info = Angle { $angle }   Power { $power }   { $weapon } ({ $ammo })   ${ $money } (+{ $income })
hud-wind = WIND
hud-wind-off = off
hud-round = Round { $round }/{ $rounds }
hud-turn = Turn { $turn }
hud-turn-sudden = Turn { $turn }  (sudden death at { $start })
hud-sudden-death = SUDDEN DEATH: every tank loses { $hp } HP each turn
hud-sudden-death-in = Sudden death in { $n } { $n ->
    [one] turn
   *[other] turns
}
hud-aiming-at = { $name } is aiming at { $target }
hud-controls = Left/Right: angle   Up/Down: power   Shift: fine   Tab: weapon   Space: FIRE   L: log   Esc: menu
bots-only-speed = Bots only: { $speed } speed
skip-to-end = Skip to end [Enter]
round-won = { $you ->
    [yes] You win the round!
   *[no] { $name } wins the round!
}
round-draw = Nobody survived. Draw!
reason-revenge = revenge
reason-leader = leader
reason-weakest = weakest
reason-nearest = nearest

## Battle log

log-fires = { $you ->
    [yes] You fire
   *[no] { $name } fires
} { $weapon }: angle { $angle }, power { $power }
log-miss = {"  "}- miss
log-hit = { $name } hit { $target }  -{ $dmg }
log-hit-self = { $you ->
    [yes] You hit yourself
   *[no] { $name } hit itself
}  -{ $dmg }
log-destroyed = { $name } destroyed { $target }
log-destroyed-self = { $you ->
    [yes] You destroyed yourself
   *[no] { $name } destroyed itself
}
log-fled = { $you ->
    [yes] You fled the battle
   *[no] { $name } fled the battle
}
log-self-destructed = { $you ->
    [yes] You self-destructed!
   *[no] { $name } self-destructed!
}
log-sudden-death = Sudden death begins!
log-targets = { $name } targets { $target } ({ $reason })
history-title = Battle log: round { $round }
history-empty = Nothing has happened yet.

## Pause menu

pause-title = PAUSED
pause-this-game = This game
pause-resume = Resume
pause-flee = Flee this round
pause-self-destruct = Self-destruct
pause-battle-log = Battle log [L]
pause-save-quit = Save & quit game
pause-note-not-aiming = Flee and Self-destruct are available only while you're aiming.
pause-note-actions = Flee: lost round, tank survives. Self-destruct: big blast, you die.
pause-note-mid-shot = Mid-shot: saving keeps the start of this turn.
abandon-title = Abandon this game?
abandon-text = This ends the game for good and deletes the save. Rounds already played stay in your stats.

## Between rounds

shop-title = Shop
shop-title-for = Shop: { $name }
shop-money = Money: ${ $money }
shop-buy = { $weapon } x{ $bundle }  -  ${ $price }
shop-done = Done - next round
inventory-title = Inventory
inventory-death-blast = Ammo aboard cooks off if your tank dies: death blast radius { $radius }, damage { $damage }.
income-title = Last round
income-total = Total
income-damage = Damage dealt
income-kills = Kills
income-bounties = Bounties
income-round-win = Round win
income-round-pay = Round pay
income-loss-bonus = Loss bonus
income-note-losses = Lost { $n } in a row. Another loss pays a ${ $bonus } loss bonus.
income-note-bounty = You have { $n } { $n ->
    [one] win
   *[other] wins
}: killing you pays a ${ $bounty } bounty.
income-note-first = Last place: you shoot first next round.
ammo-unlimited = ∞
weapon-baby-missile = Baby Missile
weapon-missile = Missile
weapon-mirv = MIRV
weapon-nuke = Nuke
weapon-dirt-bomb = Dirt Bomb

## Final standings

final-title = FINAL STANDINGS
final-play-again = Play again
final-main-menu = Main menu
ladder-up = Ladder: difficulty up to { $level }
ladder-down = Ladder: difficulty down to { $level }

## Updates

update-ask = Check for new versions of Scorch automatically when it starts?
update-yes = Yes
update-no = No
update-available = Version { $version } is available.
update-install = Update
update-later = Later
update-downloading = Downloading version { $version }...
update-installed = Version { $version } is installed.
update-restart = Restart
update-failed = Update failed: { $error }
update-checking = Checking for updates...
update-up-to-date = You have the latest version.
update-check-failed = Couldn't check for updates. Are you online?
about-check-updates = Check for updates
settings-updates = Check for updates

## Items

shop-items = Items
item-parachute = Parachute
item-shield = Shield
item-heavy-shield = Heavy Shield
item-mag-deflector = Mag Deflector
log-shield-up = { $you ->
    [yes] You raise a { $item }
   *[no] { $name } raises a { $item }
}
log-shield-absorbed = {"  "}(shield -{ $absorbed })
log-shield-down = { $you ->
    [yes] Your shield is down
   *[no] { $name }'s shield is down
}
log-parachute = { $you ->
    [yes] Your parachute opens
   *[no] { $name }'s parachute opens
}
