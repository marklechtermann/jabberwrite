//! Short sounds when a recording starts or stops (spec 0016).
//! The tones are synthesized, so no audio files ship with the app.

use std::f32::consts::TAU;
use std::sync::Arc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cue {
    /// Rising two-note chime.
    Start,
    /// Falling two-note chime.
    Stop,
}

const LOW_HZ: f32 = 660.0;
const HIGH_HZ: f32 = 990.0;
const NOTE_SECS: f32 = 0.08;
const VOLUME: f32 = 0.2;
/// Fade in and out per note, so the speaker does not click.
const FADE_SECS: f32 = 0.008;

/// Play the cue on the default output device without blocking.
/// Failures are logged only; a missing sound must never break dictation.
pub fn play(cue: Cue) {
    let spawned = std::thread::Builder::new()
        .name("cue".into())
        .spawn(move || {
            if let Err(e) = play_blocking(cue) {
                tracing::warn!("sound cue: {e}");
            }
        });
    if let Err(e) = spawned {
        tracing::warn!("sound cue: {e}");
    }
}

fn play_blocking(cue: Cue) -> Result<(), String> {
    let device = cpal::default_host()
        .default_output_device()
        .ok_or("no audio output device")?;
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let samples = Arc::new(tone(cue, config.sample_rate));
    let secs = samples.len() as f32 / config.sample_rate as f32;
    let stream = match format {
        SampleFormat::F32 => build::<f32>(&device, &config, samples),
        SampleFormat::I16 => build::<i16>(&device, &config, samples),
        SampleFormat::U16 => build::<u16>(&device, &config, samples),
        SampleFormat::I32 => build::<i32>(&device, &config, samples),
        other => return Err(format!("unsupported sample format {other}")),
    }?;
    stream.play().map_err(|e| e.to_string())?;
    // Keep the stream alive until the device buffer has drained.
    std::thread::sleep(Duration::from_secs_f32(secs) + Duration::from_millis(200));
    Ok(())
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    samples: Arc<Vec<f32>>,
) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = config.channels as usize;
    let mut pos = 0;
    device
        .build_output_stream::<T, _, _>(
            *config,
            move |data: &mut [T], _| {
                for frame in data.chunks_mut(channels) {
                    let s = samples.get(pos).copied().unwrap_or(0.0);
                    pos += 1;
                    frame.fill(T::from_sample(s));
                }
            },
            |e| tracing::warn!("sound cue stream: {e}"),
            None,
        )
        .map_err(|e| e.to_string())
}

/// Mono samples of the cue at `sample_rate`.
fn tone(cue: Cue, sample_rate: u32) -> Vec<f32> {
    let notes = match cue {
        Cue::Start => [LOW_HZ, HIGH_HZ],
        Cue::Stop => [HIGH_HZ, LOW_HZ],
    };
    let rate = sample_rate as f32;
    let len = (NOTE_SECS * rate) as usize;
    let fade = (FADE_SECS * rate).max(1.0);
    notes
        .iter()
        .flat_map(|&hz| {
            (0..len).map(move |i| {
                let envelope = (i as f32 / fade).min((len - 1 - i) as f32 / fade).min(1.0);
                VOLUME * envelope * (TAU * hz * i as f32 / rate).sin()
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tone_is_two_quiet_notes_without_clicks() {
        for cue in [Cue::Start, Cue::Stop] {
            let samples = tone(cue, 48_000);
            let note = (NOTE_SECS * 48_000.0) as usize;
            assert_eq!(samples.len(), 2 * note);
            assert!(samples.iter().all(|s| s.abs() <= VOLUME));
            // Each note starts and ends silent.
            for edge in [0, note - 1, note, 2 * note - 1] {
                assert!(samples[edge].abs() < 1e-3, "{cue:?} sample {edge}");
            }
        }
    }

    #[test]
    fn start_rises_and_stop_falls() {
        // Count zero crossings in the first note: the higher pitch has more.
        let crossings = |s: &[f32]| {
            s.windows(2)
                .filter(|w| w[0].signum() != w[1].signum())
                .count()
        };
        let note = (NOTE_SECS * 48_000.0) as usize;
        let start = tone(Cue::Start, 48_000);
        let stop = tone(Cue::Stop, 48_000);
        assert!(crossings(&start[..note]) < crossings(&start[note..]));
        assert!(crossings(&stop[..note]) > crossings(&stop[note..]));
    }
}
