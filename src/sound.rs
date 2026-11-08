//! Sound effects, made in code when the game starts in the style of an 8-bit console (pulse, triangle
//! and noise voices), with no sound files to ship. The battle only lists what happened (`Sfx`); the main loop plays it.

use std::sync::atomic::{AtomicBool, Ordering};

use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound};

/// Set when there's no working sound device (the audio thread gave up); the game then stays silent.
pub static AUDIO_OFF: AtomicBool = AtomicBool::new(false);

const RATE: u32 = 44_100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sfx {
    Fire,
    Blast,
    BigBlast,
    Dirt,
    Shield,
    Parachute,
    Burn,
    Dig,
    Click,
    RoundEnd,
}

const ALL: [Sfx; 10] = [
    Sfx::Fire,
    Sfx::Blast,
    Sfx::BigBlast,
    Sfx::Dirt,
    Sfx::Shield,
    Sfx::Parachute,
    Sfx::Burn,
    Sfx::Dig,
    Sfx::Click,
    Sfx::RoundEnd,
];

pub struct Sounds {
    bank: Vec<Option<Sound>>,
}

impl Sounds {
    pub async fn load() -> Self {
        let mut bank = Vec::new();
        for sfx in ALL {
            let sound = if AUDIO_OFF.load(Ordering::Relaxed) {
                None
            } else {
                load_sound_from_bytes(&wav(&synth(sfx))).await.ok()
            };
            bank.push(sound);
        }
        Self { bank }
    }

    /// Plays `sfx` at `volume` (0 = silent).
    pub fn play(&self, sfx: Sfx, volume: f32) {
        if volume <= 0.0 || AUDIO_OFF.load(Ordering::Relaxed) {
            return;
        }
        let index = ALL.iter().position(|&s| s == sfx).unwrap_or(0);
        if let Some(sound) = &self.bank[index] {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: false,
                    volume,
                },
            );
        }
    }
}

/// How loud each effect is next to the others (the loudest peaks at 0.8): explosions lead, the sounds
/// you hear all the time (clicks, fire, digging) stay in the background.
fn gain(sfx: Sfx) -> f32 {
    match sfx {
        Sfx::BigBlast => 1.0,
        Sfx::Blast => 0.85,
        Sfx::Fire | Sfx::Dirt => 0.6,
        Sfx::Shield | Sfx::RoundEnd => 0.5,
        Sfx::Parachute | Sfx::Burn | Sfx::Dig => 0.4,
        Sfx::Click => 0.25,
    }
}

/// The samples of one effect, -1..1. Made like an 8-bit console: pulse and triangle waves and a
/// stepped noise channel, with pitch and volume changing once per 1/60 s frame and volume in 16 steps.
fn synth(sfx: Sfx) -> Vec<f32> {
    let mut noise = Noise(0x9E37_79B9 ^ sfx as u32);
    let mut chip = Chip::default();
    let mut out = match sfx {
        // A short downward zap.
        Sfx::Fire => chip.render(0.16, |c, t| {
            c.pulse(0.25, 900.0 * (-t * 14.0).exp() + 140.0, fade(t, 0.16))
        }),
        // Falling noise: the classic 8-bit explosion, with a low triangle thump under it.
        Sfx::Blast => chip.render(0.7, |c, t| {
            0.8 * c.noise(&mut noise, 1800.0 * (-t * 5.0).exp() + 250.0, fade(t, 0.7))
                + 0.5 * c.triangle(110.0 * (-t * 6.0).exp() + 40.0, fade(t, 0.4))
        }),
        Sfx::BigBlast => chip.render(1.5, |c, t| {
            0.8 * c.noise(&mut noise, 1200.0 * (-t * 2.5).exp() + 120.0, fade(t, 1.5))
                + 0.6 * c.triangle(80.0 * (-t * 3.0).exp() + 30.0, fade(t, 0.9))
        }),
        // A dull low thud.
        Sfx::Dirt => chip.render(0.22, |c, t| {
            0.6 * c.noise(&mut noise, 350.0, fade(t, 0.18))
                + 0.6 * c.triangle(90.0 - 150.0 * t, fade(t, 0.22))
        }),
        // A quick rising arpeggio, like a power-up.
        Sfx::Shield => chip.render(0.24, |c, t| {
            let notes = [659.25, 830.61, 987.77, 1318.5];
            let note = notes[((t / 0.05) as usize).min(notes.len() - 1)];
            c.pulse(0.125, note, fade(t, 0.24))
        }),
        // A soft upward slide.
        Sfx::Parachute => chip.render(0.3, |c, t| {
            c.pulse(0.5, 260.0 + 500.0 * t, fade(t, 0.3).min(t * 20.0))
        }),
        // Crackle: short noise bursts at random volume, fading out.
        Sfx::Burn => chip.render(0.8, |c, t| {
            let level = c.frame_random(&mut noise) * fade(t, 0.8);
            c.noise(&mut noise, 3000.0, level)
        }),
        // A low chugging rumble.
        Sfx::Dig => chip.render(0.45, |c, t| {
            let chug = if (t * 16.0).fract() < 0.5 { 1.0 } else { 0.4 };
            c.noise(&mut noise, 220.0, chug * fade(t, 0.45))
        }),
        // A single blip.
        Sfx::Click => chip.render(0.035, |c, t| c.pulse(0.5, 1046.5, fade(t, 0.035))),
        // A little fanfare: melody on a pulse wave, bass on the triangle.
        Sfx::RoundEnd => {
            let melody = [(523.25, 0.0), (659.25, 0.1), (783.99, 0.2), (1046.5, 0.3)];
            let bass = [(130.81, 0.0), (196.0, 0.2), (261.63, 0.3)];
            let at = |notes: &[(f32, f32)], t: f32| {
                notes
                    .iter()
                    .rev()
                    .find(|n| t >= n.1)
                    .map_or((0.0, 0.0), |n| (n.0, t - n.1))
            };
            chip.render(0.75, |c, t| {
                let (m, mt) = at(&melody, t);
                let (b, bt) = at(&bass, t);
                let hold = if t < 0.3 { 0.09 } else { 0.45 };
                0.6 * c.pulse(0.25, m, fade(mt, hold)) + 0.5 * c.triangle(b, fade(bt, 0.45))
            })
        }
    };
    // Take the sharpest edge off the square waves, so they buzz less.
    lowpass(&mut out, 6000.0);
    normalize(&mut out, 0.8 * gain(sfx));
    out
}

/// 1 at the start, falling to 0 at `length` seconds (a straight line, then quantised by `Chip`).
fn fade(t: f32, length: f32) -> f32 {
    (1.0 - t / length).clamp(0.0, 1.0)
}

/// The 8-bit voices. Pitch and volume are read once per 1/60 s frame, and volume is kept to 16 steps, which
/// gives the stepped sound of old consoles. Each voice keeps its own phase so pitch changes don't click.
#[derive(Default)]
struct Chip {
    /// Seconds into the effect.
    time: f32,
    /// Frame the held values belong to (none yet: the first sample sets them).
    frame: Option<u32>,
    held: Vec<(f32, f32)>,
    /// Index of the next voice called within this sample.
    voice: usize,
    phases: Vec<f32>,
    noise_bit: f32,
    random: f32,
}

const FRAME: f32 = 1.0 / 60.0;

impl Chip {
    fn render(&mut self, seconds: f32, mut f: impl FnMut(&mut Chip, f32) -> f32) -> Vec<f32> {
        let n = (seconds * RATE as f32) as usize;
        (0..n)
            .map(|i| {
                self.time = i as f32 / RATE as f32;
                let frame = (self.time / FRAME) as u32;
                let new_frame = self.frame != Some(frame);
                if new_frame {
                    self.frame = Some(frame);
                    self.held.clear();
                }
                self.voice = 0;
                // Call `f` at the frame start to latch the values, then reuse them.
                let frame_time = frame as f32 * FRAME;
                f(self, frame_time)
            })
            .collect()
    }

    /// The (frequency, volume) held for this voice during the current frame, and its phase step.
    fn step(&mut self, freq: f32, volume: f32) -> (f32, f32) {
        let v = self.voice;
        self.voice += 1;
        if self.held.len() <= v {
            let volume = (volume.clamp(0.0, 1.0) * 15.0).round() / 15.0;
            self.held.push((freq.max(1.0), volume));
        }
        if self.phases.len() <= v {
            self.phases.push(0.0);
        }
        let (freq, volume) = self.held[v];
        let before = self.phases[v];
        self.phases[v] = (before + freq / RATE as f32).fract();
        (before, volume)
    }

    /// Square-ish wave; `duty` is the share of each cycle spent high (0.125, 0.25 or 0.5 on old chips).
    fn pulse(&mut self, duty: f32, freq: f32, volume: f32) -> f32 {
        let (phase, volume) = self.step(freq, volume);
        if phase < duty { volume } else { -volume }
    }

    /// Triangle stepped to 16 levels, like the bass channel of old consoles.
    fn triangle(&mut self, freq: f32, volume: f32) -> f32 {
        let (phase, volume) = self.step(freq, volume);
        let tri = 1.0 - 4.0 * (phase - 0.5).abs();
        (tri * 7.5).round() / 7.5 * volume
    }

    /// Noise that changes value `freq` times a second: low rates rumble, high rates hiss.
    fn noise(&mut self, noise: &mut Noise, freq: f32, volume: f32) -> f32 {
        let v = self.voice;
        let (phase, volume) = self.step(freq, volume);
        if self.phases[v] < phase || self.time == 0.0 {
            // The phase wrapped: a new noise value.
            self.noise_bit = if noise.next() > 0.0 { 1.0 } else { -1.0 };
        }
        self.noise_bit * volume
    }

    /// A random 0..1 that stays the same for the whole frame.
    fn frame_random(&mut self, noise: &mut Noise) -> f32 {
        if self.voice == 0 && self.held.is_empty() {
            self.random = (noise.next() + 1.0) / 2.0;
        }
        self.random
    }
}

/// One-pole low-pass filter: rounds off the sharpest edges.
fn lowpass(samples: &mut [f32], cutoff: f32) {
    let k = 1.0 - (-std::f32::consts::TAU * cutoff / RATE as f32).exp();
    let mut y = 0.0;
    for s in samples {
        y += k * (*s - y);
        *s = y;
    }
}

fn normalize(samples: &mut [f32], peak: f32) {
    let max = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
    if max > 0.0 {
        for s in samples {
            *s *= peak / max;
        }
    }
}

/// Fixed-seed noise, so every effect sounds the same each time.
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        self.0 as f32 / u32::MAX as f32 * 2.0 - 1.0
    }
}

/// A mono 16-bit WAV file of `samples`.
fn wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 2).to_le_bytes()); // bytes per second
    out.extend_from_slice(&2u16.to_le_bytes()); // bytes per frame
    out.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16).to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_effect_is_audible_and_in_range() {
        for sfx in ALL {
            let s = synth(sfx);
            assert!(!s.is_empty(), "{sfx:?}");
            let peak = s.iter().fold(0.0_f32, |m, v| m.max(v.abs()));
            let want = 0.8 * gain(sfx);
            assert!(
                (peak - want).abs() < 1e-3,
                "{sfx:?} peaks at {peak}, not {want}"
            );
            assert!(s.iter().all(|v| v.is_finite()), "{sfx:?}");
        }
    }

    /// Writes every effect as a WAV file, to listen to while tuning:
    /// `SCORCH_SOUNDS=<dir> cargo test write_sounds -- --ignored`.
    #[test]
    #[ignore]
    fn write_sounds() {
        let dir = std::env::var("SCORCH_SOUNDS").expect("set SCORCH_SOUNDS to a folder");
        std::fs::create_dir_all(&dir).unwrap();
        for sfx in ALL {
            std::fs::write(format!("{dir}/{sfx:?}.wav"), wav(&synth(sfx))).unwrap();
        }
    }

    #[test]
    fn effects_sound_the_same_every_time() {
        assert_eq!(synth(Sfx::Blast), synth(Sfx::Blast));
    }

    #[test]
    fn wav_header_matches_the_samples() {
        let w = wav(&[0.0, 1.0, -1.0]);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(&w[8..16], b"WAVEfmt ");
        assert_eq!(u32::from_le_bytes(w[40..44].try_into().unwrap()), 6);
        assert_eq!(w.len(), 44 + 6);
        assert_eq!(i16::from_le_bytes([w[46], w[47]]), i16::MAX);
    }
}
