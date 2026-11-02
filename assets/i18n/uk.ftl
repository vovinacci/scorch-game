## Scorch: українська. Ключі мають збігатися з en-GB.ftl (це перевіряє тест).
## Аргументи `you` / `target_you` дорівнюють "yes", коли гравець - єдина
## людина в грі ("Ви"), для граматики.
## Імена гравців стоять у називному відмінку, тому речення побудовані так,
## щоб їх не відмінювати ("влучає в танк Альфа").

## Спільне

back = Назад
done = Готово
cancel = Скасувати
on = Увімк.
off = Вимк.
save-exit-menu = Зберегти й вийти
abandon-game = Покинути гру
abandon = Покинути
unit-wins = { $n } { $n ->
    [one] перемога
    [few] перемоги
   *[many] перемог
}
unit-kills = { $n } { $n ->
    [one] знищення
    [few] знищення
   *[many] знищень
}

## Гравці

player-you = Ви
player-n = Гравець { $n }
bot-alpha = Альфа
bot-bravo = Браво
bot-charlie = Чарлі
skill-moron = Бовдур
skill-shooter = Стрілець
skill-sniper = Снайпер
skill-custom = Власний

## Заставка і «Про гру»

dedication-1 = Пам'яті Йосифа Рівкінда
dedication-years = 1952–2026
dedication-2 = людини, яка навчила мене постійно вчитися
about-title = ПРО ГРУ
about-made-with = Зроблено з ♥ Володимиром Щербініним (vovin) на Rust з macroquad.
about-font = Шрифт: Scorch Mono на основі Terminus Димитра Жекова, SIL Open Font License.
about-version = Версія

## Головне меню

tagline = танки, пагорби й забагато вибухівки
menu-continue = Продовжити
menu-new-game = Нова гра
menu-statistics = Статистика
menu-settings = Налаштування
menu-about = Про гру
menu-quit = Вийти
tribute = Данина грі Scorched Earth (1991) Венделла Гікса, «The Mother of All Games»
crash-last-time = { $n ->
    [1] Гра аварійно завершилась. Надішліть, будь ласка, звіт.
    [few] Гра аварійно завершувалась { $n } рази. Надішліть, будь ласка, звіти.
   *[many] Гра аварійно завершувалась { $n } разів. Надішліть, будь ласка, звіти.
}
crash-copy = { $n ->
    [1] Копіювати звіт
   *[other] Копіювати звіти
}
crash-dismiss = Закрити
crash-copied = { $n ->
    [1] Звіт скопійовано в буфер обміну. Вставте його в повідомлення.
   *[other] Звіти ({ $n }) скопійовано в буфер обміну. Вставте їх у повідомлення.
}

## Нова гра

setup-title = НОВА ГРА
setup-match = Матч
setup-world = Світ
setup-players = Гравці
setup-hotseat = 2 (по черзі)
setup-bots = Боти
setup-difficulty = Складність
setup-edit-custom = Налаштувати власного бота
setup-ladder = Драбина
setup-rounds = Раунди
setup-power-limit = Здоров'я обмежує силу
setup-sudden-death = Раптова смерть
setup-turn-order = Черговість
TurnOrder-LosersFirst = Спершу відстаючі
TurnOrder-WinnersFirst = Спершу лідери
TurnOrder-Random = Випадково
TurnOrder-RoundRobin = По колу
setup-terrain = Місцевість
setup-wind = Вітер
setup-wind-changes = Вітер змінюється щоходу
setup-gravity = Гравітація
setup-walls = Стіни
setup-sky = Небо
setup-lucky = Мені пощастить
setup-start = Почати гру
setup-preview-walls = стіни: { $walls }
difficulty-easy = Легко
difficulty-normal = Нормально
difficulty-hard = Важко
difficulty-brutal = Жорстоко
difficulty-custom = Власний
difficulty-points = { $points } проти { $bots }
difficulty-note = очки загрози: { $points }   (Бовдур 1, Стрілець 2, Снайпер 3; склад випадковий)
difficulty-custom-note = кожен бот: { $summary }
bot-summary = перший промах до { $angle }° / { $power } сили, зберігає { $keep }% похибки за постріл, { $hit }
bot-hits-by = зазвичай влучає з { $n }-го пострілу
bot-rarely-hits = рідко влучає

SuddenDeath-Off = Вимк.
SuddenDeath-After8 = По 8 пострілів
SuddenDeath-After12 = По 12 пострілів
SuddenDeath-After16 = По 16 пострілів
SuddenDeath-After24 = По 24 постріли
TerrainKind-Rolling = Пагорби
TerrainKind-Mountains = Гори
TerrainKind-Flat = Рівнина
TerrainKind-Canyon = Каньйон
TerrainKind-Random = Випадково
WindLevel-Off = Немає
WindLevel-Calm = Слабкий
WindLevel-Normal = Звичайний
WindLevel-Strong = Сильний
GravityLevel-Low = Низька
GravityLevel-Normal = Звичайна
GravityLevel-High = Висока
Walls-Open = Відкриті
Walls-Solid = Тверді
Walls-Bouncy = Пружні
Walls-Wrap = Наскрізні
Sky-Night = Ніч
Sky-Sunset = Захід сонця
Sky-Day = День

## Підсумок гри (меню паузи)

summary-players = { $humans } проти { $bots } { $bots ->
    [one] бота
   *[many] ботів
}
summary-round = Раунд
summary-round-of = { $round } з { $rounds }
summary-turn = Хід
summary-wind-changes = { $wind }, змінний

## Власний бот

custom-title = ВЛАСНИЙ БОТ
custom-angle-error = Похибка кута першого пострілу E (°)
custom-power-error = Похибка сили першого пострілу E (сила)
custom-learning = Навчання L (частка похибки, що лишається)
custom-start-from = Почати з:
custom-explanation = Як бот цілиться:
    1. Він моделює близько 2000 пострілів з реальним вітром і пагорбами й знаходить ідеальний.
    2. Першим пострілом по новій цілі він навмисно промахується: похибка = випадково від 40% до 100% E, у будь-який бік.
    3. Після кожного пострілу: наступна похибка = похибка x L + тремтіння. Тремтіння випадкове, в межах +-12% E.
    Тож типовий промах на n-му пострілі = 0,7 x E x L^(n-1).
    L = 0,5 щоходу вдвічі зменшує промах. L = 0,9 майже не вчиться. L = 0,1 смертельний з 2-го пострілу.
    Тремтіння не зникає повністю (тремтіння надовго, праворуч).
custom-typical-miss = Типовий промах по одній цілі
custom-shot = Постріл { $n }
custom-miss = { $angle }°  { $power } сили
custom-wobble = Тремтіння надовго: +-{ $angle }°, +-{ $power } сили
custom-verdict-first = Влучає з першого пострілу. Жорстоко.
custom-verdict-by = Зазвичай влучає з { $n }-го пострілу.
custom-verdict-never = Рідко підбирається близько. Легка здобич.
custom-on-target = (влучання = похибка менше { $angle }° і { $power } сили)

## Налаштування

settings-title = НАЛАШТУВАННЯ
settings-display = Екран
settings-gameplay = Гра
settings-controls = Керування
settings-language = Мова / Language
settings-health = Здоров'я танка
settings-shot-path = Слід пострілу
settings-battle-log = Журнал бою
settings-fullscreen = Повний екран
settings-game-speed = Швидкість гри
settings-bot-speed = Швидкість лише ботів
Language-English = English
Language-Ukrainian = Українська
HealthDisplay-Bar = Смужка
HealthDisplay-BarAndNumber = Смужка + число
HealthDisplay-Number = Число
GameSpeed-Normal = Звичайна
GameSpeed-Fast = Швидко (1,5x)
GameSpeed-Faster = Швидше (2x)
BotSpeed-X2 = 2x
BotSpeed-X4 = 4x
BotSpeed-X8 = 8x
controls-aim-key = Вліво / Вправо
controls-aim = Повернути ствол
controls-power-key = Вгору / Вниз
controls-power = Сила пострілу
controls-fine-key = Shift (тримати)
controls-fine = Точне налаштування
controls-weapon-key = Tab
controls-weapon = Наступна зброя
controls-fire-key = Пробіл
controls-fire = Вогонь
controls-log-key = L (тримати)
controls-log = Повний журнал бою
controls-pause-key = Esc
controls-pause = Пауза / назад
controls-skip-key = Enter
controls-skip = До кінця раунду (лише боти)
controls-menu-move-key = Меню: Вгору / Вниз
controls-menu-move = Перейти між пунктами
controls-menu-change-key = Меню: Вліво / Вправо
controls-menu-change = Змінити значення
controls-menu-choose-key = Меню: Enter
controls-menu-choose = Вибрати

## Статистика

stats-title = СТАТИСТИКА ЗА ВЕСЬ ЧАС
stats-games-played = Зіграно ігор
stats-games-won = Виграно ігор
stats-games-abandoned = Покинуто ігор
stats-best-game = Найбільше виграних раундів за гру
stats-rounds-played = Зіграно раундів
stats-rounds-won = Виграно раундів
stats-kills = Знищено танків
stats-deaths = Втрачено танків
stats-rounds-fled = Втеч з бою
stats-shots = Пострілів
stats-damage = Завдано шкоди
stats-damage-per-shot = Шкода за постріл
stats-best-hit = Найсильніше влучання
stats-money = Зароблено грошей

## Бій

hud-info = Кут { $angle }   Сила { $power }   { $weapon } ({ $ammo })   ${ $money } (+{ $income })
hud-shield = { $name }: { $energy }/{ $full }
hud-wind = ВІТЕР
hud-wind-off = немає
hud-round = Раунд { $round }/{ $rounds }
hud-turn = Хід { $turn }
hud-turn-sudden = Хід { $turn }  (раптова смерть з { $start })
hud-sudden-death = РАПТОВА СМЕРТЬ: кожен танк втрачає { $hp } HP щоходу
hud-sudden-death-in = Раптова смерть через { $n } { $n ->
    [one] хід
    [few] ходи
   *[many] ходів
}
hud-aiming-at = { $name } цілиться { $target_you ->
    [yes] у ваш танк
   *[no] в танк { $target }
}
hud-controls = Вліво/Вправо: кут   Вгору/Вниз: сила   Shift: точно   Tab: зброя   Пробіл: ВОГОНЬ   L: журнал   Esc: меню
bots-only-speed = Лише боти: швидкість { $speed }
skip-to-end = До кінця раунду [Enter]
round-won = { $you ->
    [yes] Раунд ваш!
   *[no] Раунд виграє { $name }!
}
round-draw = Ніхто не вижив. Нічия!
reason-revenge = помста
reason-leader = лідер
reason-weakest = найслабший
reason-nearest = найближчий

## Журнал бою

log-fires = { $you ->
    [yes] Ви стріляєте
   *[no] { $name } стріляє
}: { $weapon }, кут { $angle }, сила { $power }
log-miss = {"  "}- мимо
log-hit = { $you ->
    [yes] Ви влучаєте
   *[no] { $name } влучає
} { $target_you ->
    [yes] у ваш танк
   *[no] в танк { $target }
}  -{ $dmg }
log-hit-self = { $you ->
    [yes] Ви влучаєте у свій танк
   *[no] { $name } влучає у свій танк
}  -{ $dmg }
log-destroyed = { $you ->
    [yes] Ви знищуєте
   *[no] { $name } знищує
} { $target_you ->
    [yes] ваш танк
   *[no] танк { $target }
}
log-destroyed-self = { $you ->
    [yes] Ваш танк знищено власним вогнем
   *[no] Танк { $name } знищено власним вогнем
}
log-fled = { $you ->
    [yes] Ви тікаєте з бою
   *[no] { $name } тікає з бою
}
log-self-destructed = { $you ->
    [yes] Ви підриваєте свій танк!
   *[no] { $name } підриває свій танк!
}
log-sudden-death = Починається раптова смерть!
log-targets = { $name } обирає ціль: { $target_you ->
    [yes] ваш танк
   *[no] танк { $target }
} ({ $reason })
history-title = Журнал бою: раунд { $round }
history-empty = Поки нічого не сталося.

## Меню паузи

pause-title = ПАУЗА
pause-this-game = Ця гра
pause-resume = Продовжити
pause-flee = Втекти з раунду
pause-self-destruct = Самознищення
pause-battle-log = Журнал бою [L]
pause-save-quit = Зберегти й закрити гру
pause-note-not-aiming = Втеча і самознищення доступні, лише коли ви цілитеся.
pause-note-actions = Втеча: раунд програно, танк цілий. Самознищення: потужний вибух, ви гинете.
pause-note-mid-shot = Снаряд у польоті: збережеться початок цього ходу.
abandon-title = Покинути цю гру?
abandon-text = Гру буде завершено назавжди, а збереження видалено. Зіграні раунди залишаться у статистиці.

## Між раундами

shop-title = Крамниця
shop-title-for = Крамниця: { $name }
shop-money = Гроші: ${ $money }
shop-buy = { $weapon } x{ $bundle }  -  ${ $price }
shop-done = Наступний раунд
inventory-title = Інвентар
inventory-death-blast = Якщо танк знищать, боєкомплект вибухне: радіус { $radius }, шкода { $damage }.
income-title = Минулий раунд
income-total = Разом
income-damage = Завдана шкода
income-kills = Знищення
income-bounties = Нагороди
income-round-win = Перемога в раунді
income-round-pay = Плата за раунд
income-loss-bonus = Бонус за поразки
income-note-losses = { $n } { $n ->
    [one] поразка
    [few] поразки
   *[many] поразок
} поспіль. Ще одна поразка дасть бонус ${ $bonus }.
income-note-bounty = У вас { $n } { $n ->
    [one] перемога
    [few] перемоги
   *[many] перемог
}: за ваше знищення платять нагороду ${ $bounty }.
income-note-first = Останнє місце: наступний раунд ви стріляєте першими.
ammo-unlimited = ∞
weapon-baby-missile = Міні-ракета
weapon-missile = Ракета
weapon-mirv = Касетна ракета
weapon-nuke = Ядерна бомба
weapon-dirt-bomb = Земляна бомба

## Підсумки

final-title = ПІДСУМКИ
final-play-again = Грати знову
final-main-menu = Головне меню
ladder-up = Драбина: складність зросла до «{ $level }»
ladder-down = Драбина: складність знизилась до «{ $level }»

## Оновлення

update-ask = Автоматично перевіряти нові версії Scorch під час запуску?
update-yes = Так
update-no = Ні
update-available = Доступна версія { $version }.
update-install = Оновити
update-later = Пізніше
update-downloading = Завантаження версії { $version }...
update-installed = Версію { $version } встановлено.
update-restart = Перезапустити
update-failed = Не вдалося оновити: { $error }
update-checking = Перевірка оновлень...
update-up-to-date = У вас найновіша версія.
update-check-failed = Не вдалося перевірити оновлення. Чи є інтернет?
about-check-updates = Перевірити оновлення
settings-updates = Перевіряти оновлення

## Предмети

shop-items = Предмети
item-parachute = Парашут
item-shield = Щит
item-heavy-shield = Важкий щит
item-mag-deflector = Магнітний відбивач
log-shield-up = { $you ->
    [yes] Ви піднімаєте захист: { $item }
   *[no] { $name } піднімає захист: { $item }
}
log-shield-absorbed = {"  "}(щит -{ $absorbed })
log-shield-down = { $you ->
    [yes] Ваш щит зламано
   *[no] Щит танка { $name } зламано
}
log-parachute = { $you ->
    [yes] Ваш парашут розкрився
   *[no] Парашут танка { $name } розкрився
}
