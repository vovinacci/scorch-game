//! Hazards: meteors and lightning.

use super::*;
use crate::world::Hazards;

#[test]
fn with_hazards_off_the_sky_stays_quiet() {
    let (mut players, mut b) = range(|_| 600.0);
    assert_eq!(b.rules.world.hazards, Hazards::Off);
    for _ in 0..500 {
        assert!(!b.hazard(&mut players));
    }
    assert!(b.meteors.is_empty() && b.bolts.is_empty());
}

#[test]
fn hazards_come_now_and_then_and_more_often_when_set_so() {
    let (mut players, mut b) = range(|_| 600.0);
    let mut events = |b: &mut Battle, setting| {
        b.rules.world.hazards = setting;
        let n = (0..1000).filter(|_| b.hazard(&mut players)).count();
        b.meteors.clear();
        b.bolts.clear();
        n
    };
    let rare = events(&mut b, Hazards::Rare);
    let often = events(&mut b, Hazards::Often);
    assert!((60..=140).contains(&rare), "about one turn in ten: {rare}");
    assert!((240..=360).contains(&often), "about three in ten: {often}");
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
    // Far from the hill, the highest point is the low tank's own top.
    b.tanks[0].health = 100.0;
    b.strike(620.0, &mut players);
    assert_eq!(b.tanks[0].health, 100.0);
    assert!(b.tanks[1].health < 100.0);
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
