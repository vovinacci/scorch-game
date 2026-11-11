//! Turns, health, deaths, sudden death, the battle log and random world settings.

use super::*;

#[test]
fn health_limits_power_only_when_enabled() {
    let players = bots(2);
    let mut rules = Setup::default().rules();
    rules.power_limit = true;
    let mut b = battle(&players, rules);
    b.tanks[0].health = 30.0;
    assert_eq!(b.max_power(&b.tanks[0]), 300.0);
    b.tanks[0].health = 1.0;
    assert_eq!(b.max_power(&b.tanks[0]), MIN_POWER_LIMIT);
    b.rules.power_limit = false;
    assert_eq!(b.max_power(&b.tanks[0]), MAX_POWER);
}

#[test]
fn self_destruct_kills_self_without_a_kill() {
    let mut players = bots(2);
    let mut b = battle(&players, Setup::default().rules());
    b.self_destruct(&mut players);
    b.skip_to_end(&mut players);
    assert!(!b.tanks[0].alive);
    assert_eq!(players[0].kills, 0, "dying to yourself is not a kill");
    assert!(players[0].round_stats.died);
}

#[test]
fn dying_by_own_shot_credits_the_last_enemy_who_hit_you() {
    let mut players = bots(2);
    let mut b = battle(&players, Setup::default().rules());
    let at = b.tanks[0].center();
    // Enemy (1) hits tank 0, then tank 0 finishes itself off.
    b.blast(at, 20.0, 30.0, Some(1), &mut players);
    b.tanks[0].health = 5.0;
    b.blast(at, 20.0, 30.0, Some(0), &mut players);
    b.phase = Phase::Settle;
    b.update(0.1, &mut players);
    assert!(!b.tanks[0].alive);
    assert_eq!(players[1].kills, 1, "enemy gets the kill");
    assert_eq!(players[0].kills, 0);
}

#[test]
fn full_log_has_every_turn() {
    let mut players = bots(3);
    let mut b = battle(&players, Setup::default().rules());
    b.skip_to_end(&mut players);
    for turn in 1..=b.turns {
        assert!(
            b.history.iter().any(|l| l.turn == turn),
            "turn {turn} missing from the log"
        );
    }
    let shots: u32 = players.iter().map(|p| p.round_stats.shots).sum();
    let fires = b
        .history
        .iter()
        .filter(|l| l.text.contains(" fires "))
        .count();
    assert_eq!(fires as u32, shots, "every shot is logged");
}

#[test]
fn full_log_records_targets_and_hits_by_turn() {
    let mut players = bots(2);
    let mut b = battle(&players, Setup::default().rules());
    b.tanks[1].revenge = Some(0);
    let profile = players[1].bot.unwrap();
    b.choose_target(1, &players, &profile);
    let at = b.tanks[1].center();
    b.blast(at, 20.0, 10.0, Some(0), &mut players);
    let lines: Vec<_> = b
        .history
        .iter()
        .map(|l| (l.turn, l.text.as_str()))
        .collect();
    assert_eq!(lines[0], (1, "B1 targets B0 (revenge)"));
    assert!(lines[1].1.starts_with("B0 hit B1"), "{lines:?}");
    assert_eq!(b.log.len(), 1, "targeting stays out of the feed");
}

#[test]
fn carried_ammo_grows_the_death_blast() {
    let mut p = Player::new("P", WHITE, None);
    p.ammo = [0; crate::weapons::WEAPON_COUNT];
    assert_eq!(death_blast(&p), (DEATH_RADIUS, DEATH_DAMAGE));
    p.ammo[3] = 1; // one Nuke
    let (r, d) = death_blast(&p);
    assert!(r > DEATH_RADIUS && d > DEATH_DAMAGE);
    p.ammo[3] = 100;
    assert_eq!(death_blast(&p), (DEATH_MAX_RADIUS, DEATH_MAX_DAMAGE));
}

#[test]
fn sudden_death_scales_with_the_number_of_tanks() {
    let mut rules = Setup::default().rules();
    rules.sudden_death = Some(12);
    assert_eq!(battle(&bots(2), rules).rules.sudden_death, Some(24));
    assert_eq!(battle(&bots(4), rules).rules.sudden_death, Some(48));
}

#[test]
fn sudden_death_drains_everyone() {
    let mut players = bots(2);
    let mut rules = Setup::default().rules();
    rules.sudden_death = Some(0);
    let mut b = battle(&players, rules);
    b.phase = Phase::Settle;
    b.update(0.1, &mut players);
    for t in &b.tanks {
        assert_eq!(t.health, 100.0 - SUDDEN_DEATH_DRAIN);
    }
}

#[test]
fn winning_human_never_counts_as_bots_only() {
    let mut players = bots(2);
    players[0] = Player::new("You", WHITE, None);
    let mut b = battle(&players, Setup::default().rules());
    let at = b.tanks[1].center();
    b.blast(at, 20.0, 200.0, Some(0), &mut players);
    b.phase = Phase::Settle;
    for _ in 0..2000 {
        assert!(!b.only_bots_left(&players));
        if b.update(1.0 / 60.0, &mut players).is_some() {
            break;
        }
    }
    assert!(b.tanks[0].alive);
}

#[test]
fn bots_only_round_always_ends() {
    let mut players = bots(3);
    let mut b = battle(&players, Setup::default().rules());
    assert!(b.only_bots_left(&players));
    b.skip_to_end(&mut players);
    assert!(matches!(b.phase, Phase::Over { .. }));
}

#[test]
fn a_hole_under_only_part_of_a_tank_doesnt_count_as_dropping_it() {
    let (players, mut b) = range(|x| {
        if (639.0..=641.0).contains(&x) {
            700.0
        } else {
            600.0
        }
    });
    b.tanks[1].pos = vec2(640.0, 600.0);
    b.undermine(600.0, 680.0, 0, &players);
    assert_eq!(b.tanks[1].last_hit_by, None, "its edges still hold it up");
}

#[test]
fn dropping_a_tank_is_logged_and_asks_for_revenge() {
    let (mut players, mut b) = range(|_| 600.0);
    b.tanks[1].pos = vec2(650.0, 600.0);
    let before = b.history.len();
    drop(&mut b, weapon("Riot Bomb"), 640.0, 0.0);
    run(&mut b, &mut players);
    assert_eq!(b.tanks[1].revenge, Some(0));
    assert_eq!(
        b.history.len(),
        before + 1,
        "one line, so the shot isn't logged as a miss"
    );
}

#[test]
fn random_world_settings_get_a_real_value_every_round() {
    let players = bots(2);
    let mut rules = Setup::default().rules();
    rules.world.wind = WindLevel::Random;
    rules.world.gravity = GravityLevel::Random;
    rules.world.walls = Walls::Random;
    rules.world.sky = Sky::Random;
    for _ in 0..20 {
        let b = battle(&players, rules);
        let w = b.rules.world;
        assert!(w.wind != WindLevel::Random && w.gravity != GravityLevel::Random);
        assert!(w.walls != Walls::Random && w.sky != Sky::Random);
        assert_eq!(b.history.len(), 1, "one line in the log says what came out");
        assert!(
            !b.random_picks(false).contains(&t!("setup-sky")),
            "the HUD leaves out what can be seen"
        );
    }
    rules.world = Setup::default().rules().world;
    assert!(
        battle(&players, rules).history.is_empty(),
        "nothing to say without Random"
    );
}

#[test]
fn random_picks_only_real_values_and_feeling_lucky_does_too() {
    use crate::choice::Choice;
    use crate::world::TerrainKind;
    for _ in 0..50 {
        assert!(!Walls::any_real().is_random());
        assert!(!Walls::Random.roll().is_random());
        assert_eq!(Walls::Bouncy.roll(), Walls::Bouncy, "a real value stays");
        let mut setup = Setup::default();
        setup.randomize();
        let w = setup.world;
        assert!(!w.terrain.is_random() && !w.wind.is_random() && !w.gravity.is_random());
        assert!(!w.walls.is_random() && !w.sky.is_random());
    }
    assert!(TerrainKind::Random.is_random() && !TerrainKind::Canyon.is_random());
}
