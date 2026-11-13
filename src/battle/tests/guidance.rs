//! Guidance items: what turns a shell, and what switches them on and off.

use super::*;
use crate::items::ITEMS;

const HEAT: usize = 5;
const HORIZONTAL: usize = 6;
const VERTICAL: usize = 7;

/// Two tanks on flat ground, tank 0 at 300 and tank 1 at 900.
fn flat() -> (Vec<Player>, Battle) {
    let (players, mut b) = range(|_| 600.0);
    b.tanks[0].pos.x = 300.0;
    b.tanks[1].pos.x = 900.0;
    (players, b)
}

/// Tank 0 fires a Baby Missile at `aim` (angle, power) with `item` switched on (tank 1 picked); returns where it
/// goes off, if it does.
fn fire_guided(
    b: &mut Battle,
    players: &mut [Player],
    item: Option<usize>,
    aim: (f32, f32),
) -> Option<Vec2> {
    b.current = 0;
    if let Some(item) = item {
        players[0].items[item] = 1;
    }
    b.tanks[0].guidance = item;
    b.tanks[0].guide_target = Some(1);
    (b.tanks[0].angle, b.tanks[0].power) = aim;
    b.fire(players);
    run(b, players).first().copied()
}

/// The same on fresh flat ground.
fn guided_shot(item: Option<usize>, aim: (f32, f32)) -> Option<Vec2> {
    let (mut players, mut b) = flat();
    fire_guided(&mut b, &mut players, item, aim)
}

/// Where tank 1 is on the flat ground.
fn target() -> Vec2 {
    flat().1.tanks[1].center()
}

/// The power at which a plain shot at 60 degrees lands `from` to `to` px past tank 1 (short of it if negative).
fn power_landing(from: f32, to: f32) -> f32 {
    (200..1000)
        .step_by(5)
        .map(|p| p as f32)
        .find(|&p| {
            guided_shot(None, (60.0, p)).is_some_and(|at| (from..to).contains(&(at.x - target().x)))
        })
        .expect("such a shot")
}

#[test]
fn the_items_are_where_the_code_expects_them() {
    assert_eq!(ITEMS[HEAT].steer(), Some(Steer::Heat));
    assert_eq!(ITEMS[HORIZONTAL].steer(), Some(Steer::Horizontal));
    assert_eq!(ITEMS[VERTICAL].steer(), Some(Steer::Vertical));
    assert_eq!(BOT_GUIDANCE, [VERTICAL, HEAT, HORIZONTAL]);
}

#[test]
fn vertical_guidance_drops_a_shell_that_flies_over_the_tank_onto_it() {
    let long = (60.0, power_landing(120.0, 200.0));
    let guided = guided_shot(Some(VERTICAL), long).unwrap();
    assert!(guided.distance(target()) < TANK_RADIUS + 2.0, "{guided}");
    // Too short: it never gets over the tank, so nothing turns it.
    let short = (60.0, power_landing(-200.0, -120.0));
    assert_eq!(guided_shot(Some(VERTICAL), short), guided_shot(None, short));
    // One per shot; the last one switches it off.
    let (mut players, mut b) = flat();
    fire_guided(&mut b, &mut players, Some(VERTICAL), long);
    assert_eq!((players[0].items[VERTICAL], b.tanks[0].guidance), (0, None));
}

#[test]
fn horizontal_guidance_flies_level_at_the_tank_from_its_height() {
    // Short and long alike come down through the tank's level and turn towards it.
    for aim in [
        (60.0, power_landing(-200.0, -120.0)),
        (60.0, power_landing(120.0, 200.0)),
    ] {
        let guided = guided_shot(Some(HORIZONTAL), aim).unwrap();
        assert!(
            guided.distance(target()) < TANK_RADIUS + 2.0,
            "{aim:?}: {guided}"
        );
    }
    // A hill in between stops it.
    let (mut players, mut b) = flat();
    b.terrain = Terrain::from_fn(|x| {
        if (700.0..800.0).contains(&x) {
            540.0
        } else {
            600.0
        }
    });
    let aim = (60.0, power_landing(-350.0, -300.0));
    let guided = fire_guided(&mut b, &mut players, Some(HORIZONTAL), aim).unwrap();
    assert!(guided.x < 800.0, "{guided}");
}

#[test]
fn heat_guidance_turns_a_near_miss_into_a_hit() {
    let near = (60.0, power_landing(-90.0, -50.0));
    let guided = guided_shot(Some(HEAT), near).unwrap();
    assert!(guided.distance(target()) < TANK_RADIUS + 2.0, "{guided}");
    // A wide miss stays one.
    let wide = (60.0, power_landing(-300.0, -200.0));
    assert_eq!(guided_shot(Some(HEAT), wide), guided_shot(None, wide));
}

#[test]
fn a_mag_deflector_switches_guidance_off() {
    let (mut players, mut b) = flat();
    players[1].items[3] = 1;
    b.raise_shields(&mut players);
    b.current = 0;
    players[0].items[VERTICAL] = 1;
    b.tanks[0].guidance = Some(VERTICAL);
    (b.tanks[0].angle, b.tanks[0].power) = (60.0, power_landing(120.0, 200.0));
    b.fire(&mut players);
    let centre = b.tanks[1].center();
    let mut turned = false;
    while let Some(shell) = b.projectiles.first() {
        let inside = shell.pos.distance(centre) < DEFLECT_RADIUS;
        turned |= shell.vel.x == 0.0 && shell.guide.is_some();
        assert!(
            !(inside && shell.guide.is_some()),
            "guided inside the field"
        );
        b.step_projectiles(&mut players);
    }
    assert!(turned, "it did turn above the tank first");
}

#[test]
fn a_tracer_flies_unguided_and_keeps_the_guidance_for_the_real_shot() {
    let (mut players, mut b) = flat();
    b.current = 0;
    players[0].items[VERTICAL] = 1;
    players[0].ammo[weapon("Tracer")] = 1;
    b.tanks[0].guidance = Some(VERTICAL);
    b.tanks[0].weapon = weapon("Tracer");
    b.fire(&mut players);
    assert!(b.projectiles[0].guide.is_none());
    assert_eq!(
        (players[0].items[VERTICAL], b.tanks[0].guidance),
        (1, Some(VERTICAL))
    );
}

#[test]
fn keys_step_through_the_guidance_owned_and_the_living_enemies() {
    let mut players = bots(4);
    let mut b = battle(&players, Setup::default().rules());
    players[0].items[HEAT] = 1;
    players[0].items[VERTICAL] = 2;
    let mut seen = Vec::new();
    for _ in 0..4 {
        b.next_guidance(0, &players[0]);
        seen.push(b.tanks[0].guidance);
    }
    assert_eq!(seen, [Some(HEAT), Some(VERTICAL), None, Some(HEAT)]);
    // The nearest enemy to begin with, then the next living one, round and round.
    b.tanks[2].alive = false;
    let first = b.guide_target(0).unwrap();
    assert!(first == 1 || first == 3);
    b.next_guide_target(0);
    let second = b.guide_target(0).unwrap();
    assert!(second != first && second != 2 && second != 0);
    b.next_guide_target(0);
    assert_eq!(b.guide_target(0), Some(first));
    // The one picked dies: back to the nearest.
    b.tanks[first].alive = false;
    assert_eq!(b.guide_target(0), Some(second));
}

#[test]
fn a_bot_switches_on_the_guidance_it_has_for_the_tank_it_is_after() {
    let (mut players, mut b) = flat();
    players[0].items[HEAT] = 1;
    players[0].items[VERTICAL] = 1;
    b.current = 0;
    b.aim(0.0, 1.0, None, &mut players);
    assert_eq!(
        (b.tanks[0].guidance, b.tanks[0].guide_target),
        (Some(VERTICAL), Some(1))
    );
    // Digging out, it keeps them.
    b.bury_all();
    b.aim(0.0, 1.0, None, &mut players);
    assert_eq!(b.tanks[0].guidance, None);
}
