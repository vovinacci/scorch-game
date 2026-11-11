//! Bots: what they fire, where they aim, how they dig out.

use super::*;

#[test]
fn bots_never_fire_utility_weapons_at_enemies() {
    let mut players = bots(2);
    players[0].ammo[weapon("Digger")] = 5;
    players[0].ammo[weapon("Dirt Bomb")] = 5;
    players[0].ammo[weapon("Roller")] = 1;
    let mut b = battle(&players, Setup::default().rules());
    for _ in 0..10 {
        let shot = b.plan_shot(0, &players);
        assert!(!WEAPONS[shot.weapon].kind.is_utility());
    }
}

/// A bot on flat ground with a Dirt Bomb dropped right on it.
fn buried_bot() -> (Vec<Player>, Battle) {
    let (players, mut b) = range(|_| 600.0);
    let at = b.tanks[0].pos;
    b.terrain.add_dirt(at, 45.0);
    (players, b)
}

/// Fires the buried bot's planned shot; true if its barrel is clear afterwards.
fn digs_out(b: &mut Battle, players: &mut [Player]) -> bool {
    let shot = b.plan_shot(0, players);
    let muzzle = b.tanks[0].barrel_base() + direction(shot.angle) * BARREL;
    let vel = launch_velocity(shot.angle, shot.power);
    b.projectiles
        .push(Projectile::new(muzzle, vel, shot.weapon, 0, false));
    run(b, players);
    !b.terrain.is_solid(muzzle)
}

#[test]
fn a_buried_bot_digs_out_with_a_riot_bomb_first() {
    let (mut players, mut b) = buried_bot();
    players[0].ammo[weapon("Riot Bomb")] = 1;
    players[0].ammo[weapon("Missile")] = 1;
    assert_eq!(b.plan_shot(0, &players).weapon, weapon("Riot Bomb"));
    assert!(digs_out(&mut b, &mut players));
    assert!((b.tanks[0].health - 100.0).abs() < 0.01, "no damage");
}

#[test]
fn a_buried_bot_spends_a_missile_to_get_out_in_one_shot() {
    let (mut players, mut b) = buried_bot();
    players[0].ammo[weapon("Missile")] = 1;
    players[0].ammo[weapon("Nuke")] = 1;
    for _ in 0..20 {
        assert_eq!(b.plan_shot(0, &players).weapon, weapon("Missile"));
    }
    assert!(digs_out(&mut b, &mut players));
    assert!(b.tanks[0].health > 60.0, "{}", b.tanks[0].health);
    // Hurt, the same Missile would cost too much of what is left: Baby Missiles it is.
    let (mut players, mut b) = buried_bot();
    players[0].ammo[weapon("Missile")] = 1;
    b.tanks[0].health = 60.0;
    assert_eq!(b.plan_shot(0, &players).weapon, 0);
    assert!(!digs_out(&mut b, &mut players), "one is not enough");
    assert!(digs_out(&mut b, &mut players), "two are");
}

#[test]
fn a_buried_bot_never_digs_out_with_its_nuke() {
    let (mut players, mut b) = buried_bot();
    players[0].ammo[weapon("Missile")] = 0;
    players[0].ammo[weapon("Nuke")] = 1;
    for _ in 0..50 {
        assert_eq!(b.plan_shot(0, &players).weapon, 0);
    }
    // With the barrel clear again it goes back to its usual choice.
    let at = b.tanks[0].pos;
    b.terrain.carve(at - vec2(0.0, 40.0), 60.0);
    let nukes = (0..50)
        .filter(|_| b.plan_shot(0, &players).weapon == weapon("Nuke"))
        .count();
    assert!(nukes > 0);
}

#[test]
fn bots_aim_rollers_and_napalm_at_where_they_end_up() {
    let terrain = Terrain::from_fn(valley);
    let landing = Landing {
        pos: vec2(500.0, valley(500.0)),
        vel: vec2(0.0, 200.0),
        ground: true,
    };
    let roll = damage_point(Kind::Roller, landing, &terrain);
    assert!(
        (roll.x - 640.0).abs() < 6.0,
        "the Roller ends in the dip: {}",
        roll.x
    );
    let fire = damage_point(Kind::Napalm, landing, &terrain);
    assert!(
        fire.x > 600.0,
        "the fire runs down towards the dip: {}",
        fire.x
    );
    assert_eq!(damage_point(Kind::Blast, landing, &terrain), landing.pos);
}
