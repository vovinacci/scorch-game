# Third-party code kept in the repository

Crates kept in the repository because the game needs a change in them. Cargo uses them in place of the published ones
through `[patch.crates-io]` in `Cargo.toml`.

## quad-snd

[quad-snd](https://crates.io/crates/quad-snd) 0.2.8 (MIT or Apache-2.0), the audio library macroquad plays sound with.
Copied as published, without its examples, with one change:

- `src/alsa_snd.rs`: `PCM_BUFFER_SIZE` is 1024 frames, not 4096. The library mixes sound a buffer at a time and the
  sound device holds one more, so with 4096 frames (93 ms each at 44.1 kHz) a sound started 0.1 to 0.2 s after its
  event. With 1024 it is about a quarter of that. Linux only; the macOS and Windows buffers are as published.

When macroquad moves to a newer quad-snd, copy that version here and make the change again, or drop this copy if the
buffer size has become a setting.
