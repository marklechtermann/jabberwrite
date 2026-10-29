//! Speech-to-text with the Orukeet model via sherpa-onnx (ADR 0002).

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use sherpa_onnx::{OfflineRecognizer, OfflineRecognizerConfig, OfflineTransducerModelConfig};

/// Pinned ONNX INT8 release of Orukeet on Hugging Face.
const MODEL_URL: &str = "https://huggingface.co/oruk/orukeet/resolve/55a984d46f68323301837194ce647c702f55facc/onnx/sherpa-onnx-orukeet-v0.1.0-int8.tar.bz2";
const MODEL_SHA256: &str = "f9191f30178cc9122ce2f023bf9fefafc822028307b0efa4caff645ba3fe8d0a";
const MODEL_DIR: &str = "orukeet-v0.1.0-int8";
const ENCODER: &str = "encoder.int8.onnx";

pub struct Engine {
    recognizer: OfflineRecognizer,
}

impl Engine {
    pub fn load(dir: &Path) -> Result<Engine, String> {
        let file = |name: &str| -> Result<Option<String>, String> {
            let path = dir.join(name);
            if !path.is_file() {
                return Err(format!("model file missing: {}", path.display()));
            }
            Ok(Some(path.to_string_lossy().into_owned()))
        };
        let mut config = OfflineRecognizerConfig::default();
        config.model_config.transducer = OfflineTransducerModelConfig {
            encoder: file(ENCODER)?,
            decoder: file("decoder.int8.onnx")?,
            joiner: file("joiner.int8.onnx")?,
        };
        config.model_config.tokens = file("tokens.txt")?;
        config.model_config.model_type = Some("nemo_transducer".into());
        config.model_config.num_threads = std::thread::available_parallelism()
            .map(|n| n.get().min(8) as i32)
            .unwrap_or(4);
        let recognizer =
            OfflineRecognizer::create(&config).ok_or("failed to load the speech model")?;
        Ok(Engine { recognizer })
    }

    /// Transcribe mono samples; sherpa-onnx resamples to 16 kHz internally.
    pub fn transcribe(&self, samples: &[f32], sample_rate: u32) -> Result<String, String> {
        let stream = self.recognizer.create_stream();
        stream.accept_waveform(sample_rate as i32, samples);
        self.recognizer.decode(&stream);
        let result = stream.get_result().ok_or("transcription failed")?;
        Ok(result.text.trim().to_string())
    }
}

/// Return the model directory, downloading the model first if needed.
/// `progress` receives a value between 0.0 and 1.0.
pub fn ensure_model(models_dir: &Path, progress: impl Fn(f32)) -> Result<PathBuf, String> {
    let target = models_dir.join(MODEL_DIR);
    if target.join(ENCODER).is_file() {
        return Ok(target);
    }
    fs::create_dir_all(models_dir).map_err(|e| e.to_string())?;
    let archive = models_dir.join(format!("{MODEL_DIR}.tar.bz2.part"));
    download(&archive, &progress)?;

    tracing::info!("extracting model");
    let staging = models_dir.join(format!("{MODEL_DIR}.extracting"));
    let _ = fs::remove_dir_all(&staging);
    let file = fs::File::open(&archive).map_err(|e| e.to_string())?;
    tar::Archive::new(bzip2::read::BzDecoder::new(file))
        .unpack(&staging)
        .map_err(|e| format!("extracting model: {e}"))?;
    let root = find_dir_with(&staging, ENCODER).ok_or("model archive has no encoder")?;
    let _ = fs::remove_dir_all(&target);
    fs::rename(&root, &target).map_err(|e| e.to_string())?;
    let _ = fs::remove_dir_all(&staging);
    let _ = fs::remove_file(&archive);
    Ok(target)
}

fn download(dest: &Path, progress: &impl Fn(f32)) -> Result<(), String> {
    tracing::info!("downloading model from {MODEL_URL}");
    let err = |e: &dyn std::fmt::Display| format!("downloading model: {e}");
    let mut response = ureq::get(MODEL_URL).call().map_err(|e| err(&e))?;
    let total = response.body().content_length().unwrap_or(0);
    let mut reader = response.body_mut().as_reader();
    let mut file = fs::File::create(dest).map_err(|e| err(&e))?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    let (mut done, mut last_pct) = (0u64, u64::MAX);
    loop {
        let n = reader.read(&mut buf).map_err(|e| err(&e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        file.write_all(&buf[..n]).map_err(|e| err(&e))?;
        done += n as u64;
        if total > 0 && done * 100 / total != last_pct {
            last_pct = done * 100 / total;
            progress(done as f32 / total as f32);
        }
    }
    file.flush().map_err(|e| err(&e))?;
    let hash = format!("{:x}", hasher.finalize());
    if hash != MODEL_SHA256 {
        let _ = fs::remove_file(dest);
        return Err(format!("model checksum mismatch (got {hash})"));
    }
    Ok(())
}

/// Find `dir` or a direct subdirectory that contains `file`.
fn find_dir_with(dir: &Path, file: &str) -> Option<PathBuf> {
    if dir.join(file).is_file() {
        return Some(dir.to_path_buf());
    }
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.join(file).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Manual check against a real model:
    /// `JABBERWRITE_TEST_MODEL=<dir> JABBERWRITE_TEST_WAV=<file> cargo test -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn transcribes_wav() {
        let model = std::env::var("JABBERWRITE_TEST_MODEL").expect("JABBERWRITE_TEST_MODEL");
        let wav = std::env::var("JABBERWRITE_TEST_WAV").expect("JABBERWRITE_TEST_WAV");
        let engine = Engine::load(Path::new(&model)).unwrap();
        let wave = sherpa_onnx::Wave::read(&wav).expect("read wav");
        let started = std::time::Instant::now();
        let text = engine
            .transcribe(wave.samples(), wave.sample_rate() as u32)
            .unwrap();
        println!("{:.2}s: {text}", started.elapsed().as_secs_f32());
        assert!(!text.is_empty());
    }
}
