//! Sound effects, made in code when the game starts: noise bursts, sweeps and tones in the spirit of the original's
//! PC speaker, with no sound files to ship. The battle only lists what happened (`Sfx`); the main loop plays it.

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

/// The samples of one effect, -1..1, peaking at 0.8 so every effect is about as loud.
fn synth(sfx: Sfx) -> Vec<f32> {
    use std::f32::consts::TAU;
    let mut noise = Noise(0x9E37_79B9 ^ sfx as u32);
    let mut out = match sfx {
        Sfx::Fire => {
            let mut boom = render(0.25, |_, _| noise.next());
            lowpass(&mut boom, 3000.0);
            let mut phase = 0.0;
            render(0.25, |t, _| {
                phase += TAU * (60.0 + 220.0 * (-t * 8.0).exp()) / RATE as f32;
                0.6 * phase.sin() * (-t * 12.0).exp()
            })
            .into_iter()
            .zip(&boom)
            .enumerate()
            .map(|(i, (tone, n))| tone + 0.5 * n * (-(i as f32 / RATE as f32) * 25.0).exp())
            .collect()
        }
        Sfx::Blast => explosion(&mut noise, 0.9, 900.0, 5.0, 55.0),
        Sfx::BigBlast => explosion(&mut noise, 1.8, 500.0, 2.5, 40.0),
        Sfx::Dirt => {
            let mut thud = render(0.35, |t, _| noise.next() * (-t * 14.0).exp());
            lowpass(&mut thud, 400.0);
            add_tone(&mut thud, 90.0, 20.0, 0.6);
            thud
        }
        Sfx::Shield => render(0.3, |t, _| {
            let f = 600.0 + 1200.0 * t;
            (TAU * f * t).sin() * (0.6 + 0.4 * (TAU * 30.0 * t).sin()) * (-t * 10.0).exp()
        }),
        Sfx::Parachute => {
            let mut whoosh = render(0.4, |_, p| noise.next() * (std::f32::consts::PI * p).sin());
            lowpass(&mut whoosh, 1500.0);
            whoosh
        }
        Sfx::Burn => {
            let mut pop = 0.0_f32;
            let mut crackle = render(1.0, |t, _| {
                let n = noise.next();
                if n > 0.997 {
                    pop = 1.0;
                }
                pop *= 0.995;
                (0.15 * n + pop * noise.next()) * (-t * 2.0).exp()
            });
            lowpass(&mut crackle, 2500.0);
            crackle
        }
        Sfx::Dig => {
            let mut rumble = render(0.6, |t, p| {
                noise.next() * (0.6 + 0.4 * (TAU * 12.0 * t).sin()) * (1.0 - p)
            });
            lowpass(&mut rumble, 250.0);
            rumble
        }
        Sfx::Click => render(0.03, |t, _| {
            let square = if (1000.0 * t).fract() < 0.5 {
                1.0
            } else {
                -1.0
            };
            square * (-t * 120.0).exp()
        }),
        Sfx::RoundEnd => {
            let notes = [523.25, 659.25, 783.99, 1046.5];
            let note = 0.11;
            render(note * notes.len() as f32 + 0.2, |t, _| {
                let k = ((t / note) as usize).min(notes.len() - 1);
                let local = t - k as f32 * note;
                let triangle = 1.0 - 4.0 * ((notes[k] * t).fract() - 0.5).abs();
                triangle * (-local * 9.0).exp()
            })
        }
    };
    normalize(&mut out, 0.8);
    out
}

fn explosion(noise: &mut Noise, seconds: f32, cutoff: f32, decay: f32, thump: f32) -> Vec<f32> {
    let mut boom = render(seconds, |t, _| {
        noise.next() * (1.0 - (-t * 200.0).exp()) * (-t * decay).exp()
    });
    lowpass(&mut boom, cutoff);
    add_tone(&mut boom, thump, decay * 1.6, 0.7);
    boom
}

/// `seconds` of samples from `f(time, progress 0..1)`.
fn render(seconds: f32, mut f: impl FnMut(f32, f32) -> f32) -> Vec<f32> {
    let n = (seconds * RATE as f32) as usize;
    (0..n)
        .map(|i| f(i as f32 / RATE as f32, i as f32 / n as f32))
        .collect()
}

/// Adds a decaying sine of `freq` Hz.
fn add_tone(samples: &mut [f32], freq: f32, decay: f32, level: f32) {
    for (i, s) in samples.iter_mut().enumerate() {
        let t = i as f32 / RATE as f32;
        *s += level * (std::f32::consts::TAU * freq * t).sin() * (-t * decay).exp();
    }
}

/// One-pole low-pass filter: keeps the rumble, dulls the hiss.
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
            assert!((peak - 0.8).abs() < 1e-3, "{sfx:?} peaks at {peak}");
            assert!(s.iter().all(|v| v.is_finite()), "{sfx:?}");
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
