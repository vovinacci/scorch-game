//! Aiming help: the coaching advice.

use super::*;

/// Fires tank 0's shot, plays the bot's turn by hand back to tank 0, and returns tank 0's advice.
fn advice_after(b: &mut Battle, players: &mut [Player], angle: f32, power: f32) -> Option<String> {
    fire_and_settle(b, players, 0, angle, power);
    // The bot's turn passes; tank 0 is up again.
    b.next_turn(players);
    assert_eq!(b.current, 0);
    b.tanks[0].coaching.clone()
}

#[test]
fn coaching_after_a_short_shot_says_more_power() {
    let (mut players, mut b) = coached(|_| 600.0, 0.0);
    let advice = advice_after(&mut b, &mut players, 45.0, 300.0).expect("advice");
    assert!(advice.contains("more power"), "{advice}");
    assert!(!advice.contains("px"), "no distances: {advice}");
}

#[test]
fn coaching_after_a_long_shot_says_less_power() {
    let (mut players, mut b) = coached(|_| 600.0, 0.0);
    let advice = advice_after(&mut b, &mut players, 60.0, 950.0).expect("advice");
    assert!(
        advice.contains("less power") || advice.contains("aim"),
        "{advice}"
    );
}

#[test]
fn coaching_says_when_damage_puts_the_enemy_out_of_reach() {
    let (mut players, mut b) = coached(|_| 600.0, 0.0);
    b.tanks[0].health = 15.0; // Health limits power: at most 150
    let advice = advice_after(&mut b, &mut players, 45.0, 150.0).expect("advice");
    assert!(
        advice.contains("out of reach") && advice.contains("damage"),
        "{advice}"
    );
}

#[test]
fn coaching_says_which_way_to_turn_when_the_angle_cant_reach() {
    let (mut players, mut b) = coached(|_| 600.0, 0.0);
    // Almost flat: no power at this angle reaches 700 px away.
    let advice = advice_after(&mut b, &mut players, 6.0, 400.0).expect("advice");
    assert!(advice.contains("aim higher"), "{advice}");
}

#[test]
fn coaching_aims_at_the_enemy_nearest_where_the_shot_came_down_the_short_way_round() {
    let (_, mut b) = coached(|_| 600.0, 0.0);
    b.rules.world.walls = Walls::Wrap;
    b.tanks[0].pos.x = 100.0;
    b.tanks[1].pos.x = 1150.0;
    assert_eq!(b.across(100.0, 1150.0), -(WIDTH as f32) + 1050.0);
    b.coach(CoachShot {
        landing: Some(vec2(60.0, 600.0)),
        from: 100.0,
        dir: -1.0,
    });
    assert_eq!(b.tanks[0].coach_target, Some(1), "130 px round the edge");
}

#[test]
fn a_miss_is_marked_even_when_another_line_follows_the_shot() {
    let (players, mut b) = coached(|_| 600.0, 0.0);
    let k = b.history.len();
    b.shot_line = Some(k);
    b.shot_hit = false;
    b.note("fires".to_string(), WHITE);
    b.note("Sudden death!".to_string(), WHITE);
    b.next_turn(&players);
    assert!(
        b.history[k].text.ends_with(&t!("log-miss")),
        "{}",
        b.history[k].text
    );
}

#[test]
fn coaching_stays_on_screen_until_the_next_shot_and_skips_the_feed() {
    let (mut players, mut b) = coached(|_| 600.0, 0.0);
    let advice = advice_after(&mut b, &mut players, 45.0, 300.0).expect("advice");
    assert!(
        b.history.iter().any(|l| l.text == advice),
        "in the full log"
    );
    assert!(!b.log.iter().any(|l| l.text == advice), "not in the feed");
    b.fire(&mut players);
    assert!(
        b.tanks[0].coaching.is_none(),
        "gone once the next shot is fired"
    );
}
