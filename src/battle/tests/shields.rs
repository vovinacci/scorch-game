//! Shields, parachutes and the Mag Deflector.

use super::*;

#[test]
fn a_shield_takes_hits_before_health() {
    let mut players = bots(2);
    players[1].items[1] = 1; // a Shield (50)
    let mut b = battle(&players, Setup::default().rules());
    b.raise_shields(&mut players);
    assert_eq!(players[1].items[1], 0, "raising uses it up");
    let at = b.tanks[1].center();
    b.blast(at, 20.0, 30.0, Some(0), &mut players);
    assert_eq!(b.tanks[1].health, 100.0);
    assert_eq!(b.tanks[1].shield.map(|s| s.energy), Some(20.0));
    b.blast(at, 20.0, 40.0, Some(0), &mut players);
    assert!(b.tanks[1].shield.is_none(), "shield is down");
    assert_eq!(b.tanks[1].health, 80.0, "the rest goes to health");
}

#[test]
fn hitting_a_shield_pays_like_hitting_health() {
    let mut players = bots(2);
    players[1].items[2] = 1; // Heavy Shield (100)
    let mut b = battle(&players, Setup::default().rules());
    b.raise_shields(&mut players);
    let at = b.tanks[1].center();
    b.blast(at, 20.0, 30.0, Some(0), &mut players);
    assert_eq!(b.tanks[1].health, 100.0, "all absorbed");
    assert_eq!(players[0].income.damage, 30, "but the shooter is paid");
}

#[test]
fn bots_see_deflectors_in_their_aim() {
    let world = Setup::default().rules().world;
    let terrain = Terrain::generate(crate::world::TerrainKind::Flat);
    let (pos, vel) = (vec2(200.0, 300.0), vec2(300.0, -200.0));
    let plain = simulate(pos, vel, 0.0, world, &terrain, &[]);
    let bent = simulate(pos, vel, 0.0, world, &terrain, &[vec2(330.0, 230.0)]);
    assert_ne!(plain, bent, "the deflector changes where the shell lands");
}

#[test]
fn switched_off_shields_stay_in_the_inventory() {
    let mut players = bots(2);
    players[0].items[2] = 1;
    players[0].items_on[2] = false;
    let mut b = battle(&players, Setup::default().rules());
    b.raise_shields(&mut players);
    assert!(b.tanks[0].shield.is_none());
    assert_eq!(players[0].items[2], 1);
}

#[test]
fn a_parachute_cancels_fall_damage() {
    let mut players = bots(2);
    players[0].items[PARACHUTE] = 1;
    let mut b = battle(&players, Setup::default().rules());
    for i in 0..2 {
        b.tanks[i].pos.y -= 150.0; // both in the air, well above the ground
    }
    // Only the landing: stop as soon as the turn moves on, before any bot fires.
    b.phase = Phase::Settle;
    for _ in 0..600 {
        if !matches!(b.phase, Phase::Settle) {
            break;
        }
        b.update(1.0 / 60.0, &mut players);
    }
    assert_eq!(b.tanks[0].health, 100.0, "the parachute opened");
    assert_eq!(players[0].items[PARACHUTE], 0, "and was used up");
    assert!(b.tanks[1].health < 100.0, "no parachute, the fall hurt");
}

#[test]
fn a_mag_deflector_pushes_enemy_shells_away() {
    let mut players = bots(2);
    players[1].items[3] = 1;
    let mut rules = Setup::default().rules();
    rules.world.terrain = crate::world::TerrainKind::Flat;
    let mut b = battle(&players, rules);
    b.raise_shields(&mut players);
    let target = b.tanks[1].center();
    // An enemy shell inside the field and above the (flat) ground, flying at the tank.
    let start = target + vec2(-60.0, -30.0);
    b.projectiles
        .push(Projectile::new(start, vec2(200.0, 0.0), 0, 0, false));
    b.step_projectiles(&mut players);
    let p = &b.projectiles[0];
    assert!(p.vel.x < 200.0, "slowed down on its way in: {}", p.vel.x);
    let full = ITEMS[3].shield_energy().unwrap();
    assert_eq!(
        b.tanks[1].shield.map(|s| s.energy),
        Some(full - DEFLECT_COST),
        "pays once"
    );
    b.step_projectiles(&mut players);
    assert_eq!(
        b.tanks[1].shield.map(|s| s.energy),
        Some(full - DEFLECT_COST),
        "not again for the same shell"
    );
}

#[test]
fn a_mag_deflector_runs_out_after_enough_shells() {
    let mut players = bots(2);
    players[1].items[3] = 1;
    let mut rules = Setup::default().rules();
    rules.world.terrain = crate::world::TerrainKind::Flat;
    let mut b = battle(&players, rules);
    b.raise_shields(&mut players);
    let target = b.tanks[1].center();
    let shells = (ITEMS[3].shield_energy().unwrap() / DEFLECT_COST) as usize;
    for _ in 0..shells {
        b.projectiles.push(Projectile::new(
            target + vec2(-60.0, -30.0),
            vec2(200.0, 0.0),
            0,
            0,
            false,
        ));
    }
    b.step_projectiles(&mut players);
    assert!(
        b.deflectors(0).is_empty(),
        "spent: new shells aren't pushed or charged"
    );
    // Once the shells are done, it goes down.
    b.drop_spent_deflectors(&players);
    assert!(b.tanks[1].shield.is_none(), "the deflector is down");
}

#[test]
fn a_mag_deflectors_last_energy_still_pushes_the_shell_that_paid_for_it() {
    let mut players = bots(2);
    players[1].items[3] = 1;
    let mut rules = Setup::default().rules();
    rules.world.terrain = crate::world::TerrainKind::Flat;
    let mut b = battle(&players, rules);
    b.raise_shields(&mut players);
    b.wind = 0.0;
    b.tanks[1].shield.as_mut().unwrap().energy = DEFLECT_COST;
    let target = b.tanks[1].center();
    b.projectiles.push(Projectile::new(
        target + vec2(-60.0, -30.0),
        vec2(200.0, 0.0),
        0,
        0,
        false,
    ));
    b.step_projectiles(&mut players);
    let p = &b.projectiles[0];
    assert!(
        p.vel.x < 200.0,
        "pushed with the last 10 energy: {}",
        p.vel.x
    );
    assert!(
        b.tanks[1].shield.is_some(),
        "still up while its shell flies"
    );

    // A second shell arriving now is neither charged nor pushed.
    let mut late = Projectile::new(target + vec2(-60.0, -30.0), vec2(200.0, 0.0), 0, 0, false);
    b.charge_deflectors(&mut late);
    assert!(
        b.deflectors_for(&late).is_empty(),
        "nothing left to push it"
    );
}

/// A battle on flat ground where tank 1 has a Mag Deflector up with `energy`, and where its centre is.
fn deflector_with(energy: f32) -> (Vec<Player>, Battle, Vec2) {
    let mut players = bots(2);
    players[1].items[3] = 1;
    let mut rules = Setup::default().rules();
    rules.world.terrain = crate::world::TerrainKind::Flat;
    let mut b = battle(&players, rules);
    b.raise_shields(&mut players);
    b.tanks[1].shield.as_mut().unwrap().energy = energy;
    let centre = b.tanks[1].center();
    (players, b, centre)
}

#[test]
fn a_mag_deflector_pays_for_every_pass_of_a_shell() {
    let (_, mut b, centre) = deflector_with(30.0);
    let inside = centre + vec2(-60.0, -30.0);
    let outside = centre + vec2(-200.0, -30.0);
    let energy = |b: &Battle| b.tanks[1].shield.map(|s| s.energy);
    let mut p = Projectile::new(inside, vec2(200.0, 0.0), 0, 0, false);
    b.charge_deflectors(&mut p);
    b.charge_deflectors(&mut p);
    assert_eq!(energy(&b), Some(20.0), "once while it stays inside");
    // Pushed out, it falls back in: a second pass.
    p.pos = outside;
    b.charge_deflectors(&mut p);
    p.pos = inside;
    b.charge_deflectors(&mut p);
    assert_eq!(energy(&b), Some(10.0), "again when it comes back");
    // Just outside the edge isn't out yet: no charge for wobbling on it.
    p.pos = centre + vec2(-(DEFLECT_RADIUS + 2.0), 0.0);
    b.charge_deflectors(&mut p);
    p.pos = inside;
    b.charge_deflectors(&mut p);
    assert_eq!(energy(&b), Some(10.0));
}

#[test]
fn the_last_charge_pushes_a_shell_through_one_pass_only() {
    let (_, mut b, centre) = deflector_with(DEFLECT_COST);
    let inside = centre + vec2(-60.0, -30.0);
    let mut p = Projectile::new(inside, vec2(200.0, 0.0), 0, 0, false);
    b.charge_deflectors(&mut p);
    assert_eq!(
        b.deflectors_for(&p),
        vec![centre],
        "pushed on the pass it paid for"
    );
    // It leaves and comes back: nothing is left to pay for a second push.
    p.pos = centre + vec2(-200.0, -30.0);
    b.charge_deflectors(&mut p);
    p.pos = inside;
    b.charge_deflectors(&mut p);
    assert!(
        b.deflectors_for(&p).is_empty(),
        "the second time it gets through"
    );
    assert_eq!(b.tanks[1].shield.map(|s| s.energy), Some(0.0));
}

#[test]
fn a_spent_deflector_absorbs_nothing_and_adds_nothing() {
    let mut players = bots(2);
    players[1].items[3] = 1;
    let mut b = battle(&players, Setup::default().rules());
    b.raise_shields(&mut players);
    b.tanks[1].shield.as_mut().unwrap().energy = -5.0;
    b.hurt(&[(1, 30.0)], Some(0), &mut players);
    assert_eq!(b.tanks[1].health, 70.0, "exactly the hit, nothing extra");
    assert!(b.tanks[1].shield_up().is_none(), "not shown while spent");
    b.drop_spent_deflectors(&players);
    assert!(b.tanks[1].shield.is_none());
}

#[test]
fn a_battery_gives_health_back_up_to_full() {
    use crate::items::BATTERY;
    let mut players = bots(2);
    players[0].items[BATTERY] = 3;
    let mut b = battle(&players, Setup::default().rules());
    assert!(
        !b.use_battery(0, &mut players),
        "nothing to mend at full health"
    );
    assert_eq!(players[0].items[BATTERY], 3, "and it isn't used up");
    b.tanks[0].health = 40.0;
    let weak = b.max_power(&b.tanks[0]);
    assert!(b.use_battery(0, &mut players));
    assert_eq!((b.tanks[0].health, players[0].items[BATTERY]), (65.0, 2));
    assert_eq!(
        b.max_power(&b.tanks[0]),
        weak + 250.0,
        "health limits power"
    );
    b.tanks[0].health = 90.0;
    assert!(b.use_battery(0, &mut players));
    assert_eq!(b.tanks[0].health, 100.0, "never past full");
    // No battery, no health; and none for the dead.
    b.tanks[1].health = 10.0;
    assert!(!b.use_battery(1, &mut players));
    b.tanks[0].health = 0.0;
    b.tanks[0].alive = false;
    assert!(!b.use_battery(0, &mut players));
    assert_eq!(players[0].items[BATTERY], 1);
}

#[test]
fn a_bot_uses_its_batteries_once_none_of_one_is_wasted() {
    use crate::items::BATTERY;
    let mut players = bots(2);
    players[0].items[BATTERY] = 3;
    let mut b = battle(&players, Setup::default().rules());
    b.current = 0;
    b.tanks[0].health = 80.0;
    b.aim(0.0, 1.0, None, &mut players);
    assert_eq!((b.tanks[0].health, players[0].items[BATTERY]), (80.0, 3));
    // Two bring it from 30 to 80; a third would give only 20 of its 25.
    b.tanks[0].health = 30.0;
    b.aim(0.0, 1.0, None, &mut players);
    assert_eq!((b.tanks[0].health, players[0].items[BATTERY]), (80.0, 1));
}
