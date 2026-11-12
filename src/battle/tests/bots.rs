//! Bots: what they fire, where they aim, how they dig out.

use super::*;
use crate::battle::bots::{Found, LOB_ANGLES, WALL_SHOT_SLACK};

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
        off_wall: false,
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

#[test]
fn a_spoiler_goes_for_the_leader_whatever_else_happens() {
    const SPOILER: usize = 3;
    let mut players = bots(3);
    let profile = PRESETS[SPOILER];
    assert_eq!(profile.name, "Spoiler");
    let mut b = three(&players);
    // Nobody has won a round yet: it behaves like any other bot, revenge first.
    b.tanks[1].revenge = Some(0);
    assert_eq!(b.choose_target(1, &players, &profile).target, 0);
    // Tank 2 leads. Tank 0 has just hit the Spoiler and is its old target, and tank 0 already aims at the leader.
    players[2].wins = 2;
    players[0].wins = 1;
    b.tanks[0].aim = Some(aim_at(2));
    b.tanks[1].aim = Some(aim_at(0));
    b.tanks[1].revenge = Some(0);
    let memory = b.choose_target(1, &players, &profile);
    assert_eq!((memory.target, memory.reason), (2, "leader"));
    // It keeps what it has learned about the leader.
    b.tanks[1].aim = Some(AimMemory {
        error: (1.0, 2.0),
        ..aim_at(2)
    });
    assert_eq!(b.choose_target(1, &players, &profile).error, (1.0, 2.0));
    // The leader is dead: the best of those left.
    b.tanks[2].alive = false;
    assert_eq!(b.choose_target(1, &players, &profile).target, 0);
    // A Spoiler that leads goes for the best of the others; two level on wins, the nearer one.
    b.tanks[2].alive = true;
    players[1].wins = 3;
    assert_eq!(b.choose_target(1, &players, &profile).target, 2);
    players[0].wins = 2;
    b.tanks[0].pos.x = 500.0;
    b.tanks[1].aim = None;
    assert_eq!(b.choose_target(1, &players, &profile).target, 0);
}

/// The shot `me` plans with a learned aim (no error) at `target`.
fn sure_shot(b: &mut Battle, players: &[Player], me: usize, target: usize) -> Shot {
    b.tanks[me].aim = Some(aim_at(target));
    b.plan_shot(me, players)
}

#[test]
fn a_tosser_lobs_its_shells_while_a_lob_gets_there() {
    const TOSSER: usize = 5;
    let (mut players, mut b) = range(|_| 600.0);
    players[0].bot = Some(PRESETS[TOSSER]);
    assert_eq!(PRESETS[TOSSER].name, "Tosser");
    // Any other bot takes the flat shot across the field when that lands closest; a Tosser never does.
    for wind in [-40.0, 0.0, 40.0] {
        b.wind = wind;
        let angle = sure_shot(&mut b, &players, 0, 1).angle as i32;
        assert!(LOB_ANGLES.contains(&angle), "wind {wind}: {angle}");
    }
    // A gale in its face: no lob gets near, so it shoots flatter and still lands close.
    b.wind = -600.0;
    let target = b.tanks[1].center();
    let lob = b.closest_shot(0, 0, target, LOB_ANGLES, false);
    assert!(lob.is_none_or(|l| l.miss > 100.0), "the lob falls short");
    let shot = sure_shot(&mut b, &players, 0, 1);
    assert!(!LOB_ANGLES.contains(&(shot.angle as i32)), "{}", shot.angle);
}

#[test]
fn a_poolshark_plays_the_walls_where_they_bounce_or_wrap() {
    let (_, mut b) = range(|_| 600.0);
    // Well away from the walls, so that the closest shot is a plain one.
    b.tanks[0].pos.x = 400.0;
    b.tanks[1].pos.x = 880.0;
    let target = b.tanks[1].center();
    let landing = |b: &Battle, shot: Found| {
        let origin = b.tanks[0].barrel_base() + direction(shot.angle) * BARREL;
        let velocity = launch_velocity(shot.angle, shot.power);
        simulate(origin, velocity, b.wind, b.world(), &b.terrain, &[]).unwrap()
    };
    for walls in [Walls::Bouncy, Walls::Wrap] {
        b.rules.world.walls = walls;
        let direct = b.closest_shot(0, 0, target, 4..=176, false).unwrap();
        let played = b.closest_shot(0, 0, target, 4..=176, true).unwrap();
        assert!(
            !landing(&b, direct).off_wall,
            "the closest shot flies straight there"
        );
        assert!(landing(&b, played).off_wall, "{walls:?}");
        assert!(played.miss <= direct.miss + WALL_SHOT_SLACK);
    }
    // Walls that stop or lose a shell: the same shot as anyone's.
    for walls in [Walls::Open, Walls::Solid] {
        b.rules.world.walls = walls;
        let direct = b.closest_shot(0, 0, target, 4..=176, false).unwrap();
        let played = b.closest_shot(0, 0, target, 4..=176, true).unwrap();
        assert_eq!((played.angle, played.power), (direct.angle, direct.power));
    }
}

#[test]
fn a_chooser_takes_a_role_for_the_round_and_shoots_as_it() {
    const TOSSER: usize = 5;
    let mut players = bots(3);
    players[1].bot = PRESETS.last().copied();
    let b = battle(&players, Setup::default().rules());
    let role = b.tanks[1].role.expect("a Chooser has a role");
    assert_ne!(role.name, "Chooser");
    assert!(b.tanks[0].role.is_none() && b.tanks[2].role.is_none());
    // Over many rounds it is not always the same one.
    let roles: std::collections::BTreeSet<&str> = (0..40)
        .filter_map(|_| battle(&players, Setup::default().rules()).tanks[1].role)
        .map(|role| role.name)
        .collect();
    assert!(roles.len() > 3, "{roles:?}");
    // As a Tosser it lobs.
    let (mut players, mut b) = range(|_| 600.0);
    players[0].bot = PRESETS.last().copied();
    b.tanks[0].role = Some(PRESETS[TOSSER]);
    let angle = sure_shot(&mut b, &players, 0, 1).angle as i32;
    assert!(LOB_ANGLES.contains(&angle), "{angle}");
    // A battle saved before there were Choosers loads without roles.
    let mut saved = serde_json::to_value(&b).unwrap();
    for tank in saved["tanks"].as_array_mut().unwrap() {
        tank.as_object_mut().unwrap().remove("role");
    }
    let loaded: Battle = serde_json::from_value(saved).unwrap();
    assert!(loaded.tanks.iter().all(|tank| tank.role.is_none()));
}
