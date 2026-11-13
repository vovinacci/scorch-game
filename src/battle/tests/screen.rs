//! What is on screen: the weapon strip, mouse aiming, text panels, the buried line, the aim preview.

use super::*;

#[test]
fn a_buried_human_is_told_while_the_barrel_is_in_the_dirt() {
    let (mut players, mut b) = range(|_| 600.0);
    players[0].bot = None;
    b.current = 0;
    b.phase = Phase::Aim {
        think: 0.0,
        plan: None,
    };
    assert!(!b.aiming_into_dirt(&players));
    let at = b.tanks[0].pos;
    b.terrain.add_dirt(at, 45.0);
    assert!(b.aiming_into_dirt(&players));
    // Not for a bot, and not once a blast has cleared the barrel.
    players[0].bot = Some(PRESETS[1]);
    assert!(!b.aiming_into_dirt(&players));
    players[0].bot = None;
    b.terrain.carve(at - vec2(0.0, 40.0), 60.0);
    assert!(!b.aiming_into_dirt(&players));
}

#[test]
fn the_weapon_strip_shows_owned_weapons_on_screen() {
    let mut players = bots(1);
    let p = &mut players[0];
    assert_eq!(
        weapon_strip(p).len(),
        2,
        "Baby Missile and the starting Missiles"
    );
    p.ammo = [1; crate::weapons::WEAPON_COUNT];
    let strip = weapon_strip(p);
    assert_eq!(strip.len(), WEAPONS.len());
    for (k, (_, a)) in strip.iter().enumerate() {
        assert!(a.w >= 140.0, "wide enough to read");
        assert!(a.left() >= 0.0 && a.right() <= ui::WIDTH && a.bottom() <= HEIGHT);
        assert!(
            strip[k + 1..].iter().all(|(_, b)| !a.overlaps(b)),
            "no overlap"
        );
    }
}

#[test]
fn the_weapon_strip_turns_see_through_over_a_tank_in_a_deep_valley() {
    let (players, mut b) = range(|_| 400.0);
    let strip = weapon_strip(&players[0]);
    assert!(!covers_a_tank(&strip, &b.tank_areas(&players)));
    // Down in a crater at the bottom of the screen, right behind the strip.
    b.tanks[1].pos = vec2(640.0, 690.0);
    assert!(covers_a_tank(&strip, &b.tank_areas(&players)));
}

#[test]
fn mouse_aiming_points_the_barrel_at_the_cursor() {
    let from = vec2(600.0, 500.0);
    let (angle, power) = point_aim(from, from + vec2(100.0, -100.0), 1000.0);
    assert!((angle - 45.0).abs() < 1e-3);
    assert!((power - 100.0 * 2.0_f32.sqrt() * POWER_PER_PIXEL).abs() < 1e-3);
    let (angle, _) = point_aim(from, from + vec2(0.0, -50.0), 1000.0);
    assert!((angle - 90.0).abs() < 1e-3, "straight up");
    let (angle, _) = point_aim(from, from + vec2(-80.0, -1.0), 1000.0);
    assert!(angle > 179.0, "almost flat to the left");
}

#[test]
fn mouse_aiming_below_the_barrel_stays_flat_and_power_is_capped() {
    let from = vec2(600.0, 500.0);
    assert_eq!(point_aim(from, from + vec2(50.0, 40.0), 1000.0).0, 0.0);
    assert_eq!(point_aim(from, from + vec2(-50.0, 40.0), 1000.0).0, 180.0);
    let (_, power) = point_aim(from, from + vec2(0.0, -600.0), 300.0);
    assert_eq!(power, 300.0, "never above the tank's maximum");
}

#[test]
fn the_aiming_preview_follows_the_real_shot() {
    let (_, b) = range(|_| 600.0);
    let (pos, vel) = (vec2(200.0, 580.0), launch_velocity(50.0, 600.0));
    let path = predict_path(pos, vel, 30.0, b.world(), &b.terrain, &[], None);
    let landing = simulate(pos, vel, 30.0, b.world(), &b.terrain, &[]).unwrap();
    assert_eq!(
        *path.last().unwrap(),
        landing.pos,
        "ends where the shot lands"
    );
    assert!(path.len() > 50);
    assert_eq!(crate::setup::AimHelp::ShortPath.preview(), 1.0 / 3.0);
    assert_eq!(crate::setup::AimHelp::Coaching.preview(), 0.0);
}

#[test]
fn text_panels_only_on_the_day_sky() {
    assert!(backing_alpha(Sky::Day) > 0.0 && backing_alpha(Sky::Day) <= 0.5);
    assert_eq!(backing_alpha(Sky::Night), 0.0);
    assert_eq!(backing_alpha(Sky::Sunset), 0.0);
}
