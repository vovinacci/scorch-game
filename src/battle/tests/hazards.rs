//! Hazards: meteors and lightning.

use super::*;
use crate::world::Hazards;

#[test]
fn with_hazards_off_the_sky_stays_quiet() {
    let (_, mut b) = range(|_| 600.0);
    assert_eq!(b.rules.world.hazards, Hazards::Off);
    for _ in 0..500 {
        assert!(!b.hazard());
    }
    assert!(b.meteors.is_empty() && b.bolts.is_empty() && b.storms.is_empty());
}

#[test]
fn hazards_come_now_and_then_and_more_often_when_set_so() {
    let (_, mut b) = range(|_| 600.0);
    let events = |b: &mut Battle, setting| {
        b.rules.world.hazards = setting;
        let n = (0..1000).filter(|_| b.hazard()).count();
        b.meteors.clear();
        b.bolts.clear();
        b.storms.clear();
        n
    };
    let rare = events(&mut b, Hazards::Rare);
    let often = events(&mut b, Hazards::Often);
    assert!(
        (35..=95).contains(&rare),
        "about one turn in sixteen: {rare}"
    );
    assert!(
        (90..=165).contains(&often),
        "about one turn in eight: {often}"
    );
}

#[test]
fn often_brings_more_at_once_than_rare() {
    let (_, mut b) = range(|_| 600.0);
    let most_at_once = |b: &mut Battle, setting| {
        b.rules.world.hazards = setting;
        let (mut meteors, mut cells) = (0, 0);
        for _ in 0..2000 {
            b.meteors.clear();
            b.storms.clear();
            b.hazard();
            meteors = meteors.max(b.meteors.len());
            cells = cells.max(b.storms.len());
        }
        (meteors, cells)
    };
    assert_eq!(most_at_once(&mut b, Hazards::Rare), (1, 1));
    assert_eq!(most_at_once(&mut b, Hazards::Often), (2, 1));
}

#[test]
fn a_meteor_goes_off_like_a_missile_and_belongs_to_nobody() {
    let (mut players, mut b) = range(|_| 600.0);
    let at = b.tanks[1].center();
    b.land_meteor(at, &mut players);
    let missile = &WEAPONS[weapon("Missile")];
    assert_eq!(b.tanks[1].health, 100.0 - missile.damage);
    assert_eq!(b.tanks[0].health, 100.0, "far away");
    assert!(b.terrain.height_at(at.x) > 600.0, "a crater");
    // Nobody is paid, credited or owed revenge.
    assert!(players.iter().all(|p| p.income.damage == 0));
    assert_eq!((b.tanks[1].last_hit_by, b.tanks[1].revenge), (None, None));
    assert!(!b.shot_hit);
    let line = &b.history.last().unwrap().text;
    assert!(line.starts_with("A meteor hits B1"), "{line}");
}

#[test]
fn a_meteor_falls_and_lands_and_a_shield_takes_it() {
    let (mut players, mut b) = range(|_| 600.0);
    players[1].items[2] = 1; // Heavy Shield
    b.raise_shields(&mut players);
    b.wind = 0.0;
    let above = vec2(b.tanks[1].pos.x, 100.0);
    b.meteors.push(crate::battle::hazards::Meteor::falling(
        above,
        vec2(0.0, 200.0),
    ));
    for _ in 0..2000 {
        b.step_meteors(&mut players);
    }
    assert!(b.meteors.is_empty(), "it came down");
    assert_eq!(b.tanks[1].health, 100.0, "the shield took it");
    assert!(b.tanks[1].shield.is_some_and(|s| s.energy < 100.0));
}

#[test]
fn lightning_strikes_the_highest_point_near_where_it_comes_down() {
    // A hill at 300 with tank 0 on top, flat ground with tank 1 at 500.
    let hill = |x: f32| 600.0 - (80.0 - (x - 300.0).abs()).max(0.0);
    let (mut players, mut b) = range(hill);
    for (tank, x) in b.tanks.iter_mut().zip([300.0, 500.0]) {
        tank.pos = vec2(x, hill(x));
    }
    // It comes down over the low tank, and finds the one on the hill.
    b.strike(420.0, &mut players);
    assert!(
        b.tanks[0].health < 100.0,
        "the tank on the hilltop is struck"
    );
    assert_eq!(b.tanks[1].health, 100.0);
    assert_eq!(b.bolts.len(), 1);
    assert_eq!((b.tanks[0].last_hit_by, b.tanks[0].revenge), (None, None));
    // Far from the hill, on flat ground, the bolt lands anywhere under the cell: the tank there is no lightning rod,
    // hurt only when the bolt happens to land close.
    b.tanks[0].health = 100.0;
    let mut hurt = 0;
    for _ in 0..200 {
        b.tanks[1].health = 100.0;
        b.strike(620.0, &mut players);
        hurt += u32::from(b.tanks[1].health < 100.0);
    }
    assert_eq!(b.tanks[0].health, 100.0, "the hill is out of reach");
    assert!(
        (10..=100).contains(&hurt),
        "now and then, not always: {hurt} of 200"
    );
}

#[test]
fn a_round_with_hazards_plays_to_its_end() {
    let mut players = bots(3);
    let mut rules = Setup::default().rules();
    rules.world.hazards = Hazards::Often;
    let mut b = battle(&players, rules);
    b.skip_to_end(&mut players);
    assert!(matches!(b.phase, Phase::Over { .. }));
    let hazards = b
        .history
        .iter()
        .filter(|l| l.text.contains("meteor") || l.text.contains("Lightning"))
        .count();
    assert!(hazards > 0, "something came down in a whole round");
}

#[test]
fn lightning_comes_from_a_storm_that_gathers_first() {
    let (mut players, mut b) = range(|_| 600.0);
    b.rules.world.hazards = Hazards::Often;
    // Until a storm comes, whatever the sky throws.
    while b.storms.is_empty() {
        b.meteors.clear();
        b.hazard();
    }
    // The cloud gathers: no bolt yet, nobody hurt, and the hazard keeps the turn waiting.
    assert!(b.hazards_busy(0.5, &mut players));
    assert!(b.bolts.is_empty());
    // Then it strikes, and clears.
    let mut struck = false;
    for _ in 0..240 {
        b.hazards_busy(1.0 / 60.0, &mut players);
        struck |= !b.bolts.is_empty();
    }
    assert!(struck, "the bolt fell once the cloud had gathered");
    assert!(
        !b.hazards_busy(0.0, &mut players),
        "and then the sky clears"
    );
}

#[test]
fn weather_sets_the_wind_and_a_storm_brings_its_own_lightning() {
    use crate::world::Weather;
    let (_, mut b) = range(|_| 600.0);
    b.rules.world.wind = WindLevel::Calm;
    assert_eq!(b.world().max_wind(), 20.0, "clear: the wind as set");
    for weather in [Weather::Windy, Weather::Storm] {
        b.rules.world.weather = weather;
        assert_eq!(b.world().max_wind(), 70.0, "{weather:?}: strong");
    }
    // Hazards off, yet a storm strikes now and then, always with lightning.
    assert_eq!(b.rules.world.hazards, Hazards::Off);
    let strikes = (0..1000)
        .filter(|_| {
            let struck = b.hazard();
            assert!(b.meteors.is_empty(), "a storm throws no meteors");
            b.storms.clear();
            struck
        })
        .count();
    assert!(
        (65..=140).contains(&strikes),
        "about one turn in ten: {strikes}"
    );
    // With Hazards on as well, both come, but never on the same turn, and the storm leaves the sky's other
    // hazards half their chance.
    b.rules.world.hazards = Hazards::Often;
    let mut meteors = 0;
    for _ in 0..2000 {
        b.meteors.clear();
        b.storms.clear();
        b.hazard();
        assert!(
            b.meteors.is_empty() || b.storms.is_empty(),
            "one thing at a time"
        );
        meteors += usize::from(!b.meteors.is_empty());
    }
    assert!(
        (40..=110).contains(&meteors),
        "meteors still come: {meteors}"
    );
    b.rules.world.hazards = Hazards::Off;
    // Random weather comes out as a real one each round.
    b.rules.world.weather = Weather::Random;
    assert!(!b.rules.world.for_round().weather.is_random());
}

#[test]
fn lightning_under_the_same_cell_strikes_different_spots() {
    // Rolling ground, no tank under the cell at 640.
    let (mut players, mut b) = range(|x| 600.0 - 40.0 * (x / 60.0).sin());
    let spots: std::collections::BTreeSet<i32> = (0..30)
        .map(|_| {
            b.bolts.clear();
            b.strike(640.0, &mut players);
            b.bolts[0].path[0].x as i32 / 10
        })
        .collect();
    assert!(spots.len() > 3, "not one peak every time: {spots:?}");
}
