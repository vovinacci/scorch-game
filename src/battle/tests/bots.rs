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

/// Three tanks in a row on flat ground, 400 px apart, tank 1 in the middle.
fn three(players: &[Player]) -> Battle {
    let mut b = battle(players, Setup::default().rules());
    for (tank, x) in b.tanks.iter_mut().zip([240.0, 640.0, 1040.0]) {
        tank.pos = vec2(x, 600.0);
    }
    b
}

fn aim_at(target: usize) -> AimMemory {
    AimMemory {
        target,
        error: (0.0, 0.0),
        reason: "nearest",
    }
}

#[test]
fn with_three_bots_left_a_bot_doesnt_square_off_with_the_one_aiming_at_it() {
    let players = bots(3);
    let mut b = three(&players);
    b.tanks[0].aim = Some(aim_at(1));
    // Both others are as near and as healthy; only tank 0 aims at tank 1.
    for _ in 0..20 {
        assert_eq!(b.score_targets(1, &players).0, 2);
    }
    // With a human among the three the rule stays out: either may be picked.
    let mut mixed = bots(3);
    mixed[2].bot = None;
    let picks: Vec<usize> = (0..40).map(|_| b.score_targets(1, &mixed).0).collect();
    assert!(picks.contains(&0) && picks.contains(&2), "{picks:?}");
}

#[test]
fn with_three_bots_left_revenge_is_left_to_the_bot_already_after_the_attacker() {
    let players = bots(3);
    let profile = players[1].bot.unwrap();
    let mut b = three(&players);
    // Tank 0 hit tank 1, which was aiming at tank 2; tank 2 is already after tank 0.
    b.tanks[2].aim = Some(aim_at(0));
    b.tanks[1].aim = Some(aim_at(2));
    b.tanks[1].revenge = Some(0);
    assert_eq!(b.choose_target(1, &players, &profile).target, 2);
    // Nobody else on the attacker: revenge as usual.
    b.tanks[2].aim = Some(aim_at(1));
    b.tanks[1].revenge = Some(0);
    assert_eq!(b.choose_target(1, &players, &profile).target, 0);
    // Four tanks: revenge as usual, even with another bot on the attacker.
    let players = bots(4);
    let mut b = battle(&players, Setup::default().rules());
    b.tanks[2].aim = Some(aim_at(0));
    b.tanks[1].aim = Some(aim_at(2));
    b.tanks[1].revenge = Some(0);
    assert_eq!(b.choose_target(1, &players, &profile).target, 0);
}
