//! Microphone capture from the OS default input device.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};

/// Mono f32 samples at the device's native sample rate.
pub struct Audio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

impl Audio {
    pub fn duration_secs(&self) -> f32 {
        self.samples.len() as f32 / self.sample_rate as f32
    }
}

/// A running recording. The cpal stream lives on its own thread because it
/// is not `Send` on every platform.
pub struct Recording {
    stop: mpsc::Sender<()>,
    done: mpsc::Receiver<Result<Audio, String>>,
}

impl Recording {
    pub fn start() -> Result<Recording, String> {
        let (stop_tx, stop_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        std::thread::Builder::new()
            .name("audio".into())
            .spawn(move || {
                let buffer = Arc::new(Mutex::new(Vec::new()));
                let (stream, sample_rate) = match open(buffer.clone()) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                let _ = ready_tx.send(Ok(()));
                let _ = stop_rx.recv();
                drop(stream);
                let samples = std::mem::take(&mut *buffer.lock().unwrap());
                let _ = done_tx.send(Ok(Audio {
                    samples,
                    sample_rate,
                }));
            })
            .map_err(|e| e.to_string())?;
        ready_rx.recv().map_err(|e| e.to_string())??;
        Ok(Recording {
            stop: stop_tx,
            done: done_rx,
        })
    }

    pub fn stop(self) -> Result<Audio, String> {
        let _ = self.stop.send(());
        self.done.recv().map_err(|e| e.to_string())?
    }
}

fn open(buffer: Arc<Mutex<Vec<f32>>>) -> Result<(cpal::Stream, u32), String> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or("no microphone found")?;
    let supported = device
        .default_input_config()
        .map_err(|e| format!("microphone: {e}"))?;
    let format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let stream = match format {
        SampleFormat::F32 => build::<f32>(&device, &config, buffer),
        SampleFormat::I16 => build::<i16>(&device, &config, buffer),
        SampleFormat::U16 => build::<u16>(&device, &config, buffer),
        SampleFormat::I32 => build::<i32>(&device, &config, buffer),
        other => return Err(format!("unsupported sample format {other}")),
    }?;
    stream.play().map_err(|e| format!("microphone: {e}"))?;
    Ok((stream, config.sample_rate))
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: Arc<Mutex<Vec<f32>>>,
) -> Result<cpal::Stream, String>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    device
        .build_input_stream::<T, _, _>(
            *config,
            move |data: &[T], _| {
                let mut buf = buffer.lock().unwrap();
                buf.extend(data.chunks(channels).map(|frame| {
                    frame.iter().map(|s| f32::from_sample_(*s)).sum::<f32>() / channels as f32
                }));
            },
            |e| tracing::error!("audio stream error: {e}"),
            None,
        )
        .map_err(|e| format!("microphone: {e}"))
}
