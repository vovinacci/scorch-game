//! Driving: where a tank can go and what it costs.

use super::bots::aim_at;
use super::*;
use crate::items::FUEL;

/// Two tanks on ground shaped by `height`, tank 0 at 300 with `fuel`, tank 1 far off at 1200.
fn field(height: impl Fn(f32) -> f32, fuel: u32) -> (Vec<Player>, Battle) {
    let (mut players, mut b) = range(height);
    for (tank, x) in b.tanks.iter_mut().zip([300.0, 1200.0]) {
        tank.pos = vec2(x, b.terrain.support_y(x, TANK_RADIUS));
    }
    players[0].items[FUEL] = fuel;
    (players, b)
}

/// Tank 0 drives `dir` for `seconds`; returns how far it got.
fn drive_for(b: &mut Battle, players: &mut [Player], dir: f32, seconds: f32) -> f32 {
    let from = b.tanks[0].pos.x;
    for _ in 0..(seconds * 60.0) as usize {
        b.drive(0, dir, 1.0 / 60.0, players);
    }
    b.tanks[0].pos.x - from
}

#[test]
fn a_tank_drives_on_level_ground_a_px_a_unit_of_fuel() {
    let (mut players, mut b) = field(|_| 600.0, 100);
    assert_eq!(drive_for(&mut b, &mut players, 1.0, 0.5).round(), 30.0);
    assert_eq!(players[0].items[FUEL], 70);
    assert_eq!(drive_for(&mut b, &mut players, -1.0, 0.5).round(), -30.0);
    // The tank runs dry and stops.
    assert_eq!(drive_for(&mut b, &mut players, 1.0, 5.0).round(), 40.0);
    assert_eq!(players[0].items[FUEL], 0);
    assert_eq!(drive_for(&mut b, &mut players, 1.0, 1.0), 0.0);
}

#[test]
fn uphill_costs_more_and_downhill_no_less() {
    // Rising to the right, a px up for every two along.
    let slope = |x: f32| 700.0 - x / 2.0;
    let (mut players, mut b) = field(slope, 1000);
    let y = b.tanks[0].pos.y;
    assert_eq!(drive_for(&mut b, &mut players, 1.0, 1.0).round(), 60.0);
    assert!(
        (y - b.tanks[0].pos.y - 30.0).abs() < 1.5,
        "it stays on the ground"
    );
    let uphill = 1000 - players[0].items[FUEL];
    assert!(
        (115..=125).contains(&uphill),
        "60 px along and 30 up: {uphill}"
    );
    players[0].items[FUEL] = 1000;
    assert_eq!(drive_for(&mut b, &mut players, -1.0, 1.0).round(), -60.0);
    assert_eq!(1000 - players[0].items[FUEL], 60);
}

#[test]
fn walls_cliffs_tanks_and_the_edge_stop_a_tank() {
    // A wall 30 px high at 340, a drop of 40 px at 260.
    let ground = |x: f32| match x {
        x if x >= 340.0 => 570.0,
        x if x < 260.0 => 640.0,
        _ => 600.0,
    };
    let (mut players, mut b) = field(ground, 1000);
    let right = drive_for(&mut b, &mut players, 1.0, 3.0);
    assert!((20.0..=30.0).contains(&right), "up to the wall: {right}");
    let left = drive_for(&mut b, &mut players, -1.0, 3.0);
    // It stands on the highest ground under it, so it stops with its last px still on the ledge.
    assert!((-82.0..=-74.0).contains(&left), "up to the drop: {left}");
    assert_eq!(b.tanks[0].pos.y, 600.0);

    // A step down it can take without harm is no obstacle.
    let (mut players, mut b) = field(|x| if x < 260.0 { 610.0 } else { 600.0 }, 1000);
    assert!(drive_for(&mut b, &mut players, -1.0, 3.0) < -150.0);
    assert_eq!(b.tanks[0].pos.y, 610.0);

    // Another tank, then the edge of the field.
    let (mut players, mut b) = field(|_| 600.0, 5000);
    b.tanks[1].pos.x = 400.0;
    let right = drive_for(&mut b, &mut players, 1.0, 5.0);
    assert!(
        (70.0..=74.0).contains(&right),
        "it stops short of the tank: {right}"
    );
    drive_for(&mut b, &mut players, -1.0, 30.0);
    assert_eq!(b.tanks[0].pos.x, TANK_RADIUS);

    // Under the dirt there is no driving.
    let (mut players, mut b) = field(|_| 600.0, 1000);
    b.bury_all();
    assert_eq!(drive_for(&mut b, &mut players, 1.0, 1.0), 0.0);
    assert_eq!(players[0].items[FUEL], 1000);
}

#[test]
fn a_bot_drives_closer_when_its_shots_fall_short() {
    let (mut players, mut b) = field(|_| 600.0, 400);
    b.current = 0;
    // Healthy, it reaches the other tank from where it is: it stays.
    assert_eq!(b.better_ground(0, &players), None);
    // Nearly dead, health limits its power to a short lob: it wants to get closer.
    b.tanks[0].health = 12.0;
    let goal = b.better_ground(0, &players).expect("somewhere better");
    assert!(goal > 400.0, "{goal}");
    assert_eq!(b.tanks[0].pos.x, 300.0, "looking moves nothing");
    // On its turn it drives there before it aims, then shoots from there.
    let mut frames = 0;
    while b.bot_drives(0, 1.0 / 60.0, &mut players) {
        frames += 1;
        assert!(frames < 2000, "it arrives");
    }
    assert!(
        (b.tanks[0].pos.x - goal).abs() < 1.5,
        "{}",
        b.tanks[0].pos.x
    );
    assert!(players[0].items[FUEL] < 400 - 100);
    assert!(!b.bot_drives(0, 1.0 / 60.0, &mut players), "once a turn");
    // Without fuel it stays where it is.
    players[0].items[FUEL] = 0;
    b.drive_checked = None;
    assert_eq!(b.better_ground(0, &players), None);
}

#[test]
fn driving_costs_a_bot_its_ranging_on_that_tank() {
    let (players, mut b) = field(|_| 600.0, 0);
    let profile = players[0].bot.unwrap();
    let most = profile.first_angle_error;
    let ranged_in = |b: &mut Battle| {
        b.tanks[0].aim = Some(AimMemory {
            ranged: Some((300.0, 1200.0)),
            ..aim_at(1)
        });
    };
    // Nobody has moved: the aim it had.
    ranged_in(&mut b);
    assert_eq!(b.choose_target(0, &players, &profile).error, (0.0, 0.0));
    // The target drove half of what it takes to shake a bot off: half the way back to a first shot's error.
    ranged_in(&mut b);
    b.tanks[1].pos.x += RERANGE / 2.0;
    let half = b.choose_target(0, &players, &profile).error.0.abs();
    assert!((0.19 * most..=0.51 * most).contains(&half), "{half}");
    // Both drove, further than that between them: a first shot again.
    ranged_in(&mut b);
    b.tanks[0].pos.x -= RERANGE;
    let all = b.choose_target(0, &players, &profile).error.0.abs();
    assert!((0.39 * most..=1.01 * most).contains(&all), "{all}");
}

#[test]
fn a_bot_that_was_hit_drives_off_unless_it_would_lose_its_own_aim() {
    let (players, mut b) = field(|_| 600.0, 300);
    b.current = 0;
    assert_eq!(b.dodge(0, &players), None, "nobody hit it");
    b.tanks[0].revenge = Some(1);
    let goal = b
        .dodge(0, &players)
        .expect("it was hit and has no aim to lose");
    assert_eq!((goal - 300.0).abs(), 90.0);
    // Ranged in on its own target, it stays and shoots.
    b.tanks[0].aim = Some(aim_at(1));
    assert_eq!(b.dodge(0, &players), None);
    // Boxed in by walls it can't climb, it stays too.
    b.tanks[0].aim = None;
    b.terrain = Terrain::from_fn(|x| {
        if (280.0..320.0).contains(&x) {
            600.0
        } else {
            560.0
        }
    });
    assert_eq!(b.dodge(0, &players), None);
    // On its turn it drives before it aims.
    let (mut players, mut b) = field(|_| 600.0, 300);
    b.current = 0;
    b.tanks[0].revenge = Some(1);
    while b.bot_drives(0, 1.0 / 60.0, &mut players) {}
    assert!((b.tanks[0].pos.x - 300.0).abs() > 88.0);
    assert!(players[0].items[FUEL] <= 210);
}

#[test]
fn a_tank_drives_on_a_fast_screen_too() {
    // 144 frames a second: less than a px each.
    let (mut players, mut b) = field(|_| 600.0, 100);
    for _ in 0..144 {
        b.drive(0, 1.0, 1.0 / 144.0, &mut players);
    }
    assert!(
        (b.tanks[0].pos.x - 360.0).abs() <= 1.0,
        "{}",
        b.tanks[0].pos.x
    );
}
