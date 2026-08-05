// On-device meeting listener: captures the mic, transcribes locally with Whisper.
// Audio never leaves the machine, and by default only the notes are kept (the
// caller discards the transcript). Requires a Whisper GGML model file that the
// user supplies — nothing is bundled or downloaded automatically.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub struct Recorder {
    stop: Arc<AtomicBool>,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    handle: Option<JoinHandle<()>>,
}

#[derive(Default)]
pub struct ListenerState(pub Mutex<Option<Recorder>>);

pub fn start() -> Result<Recorder, String> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or("No microphone found.")?;
    let supported = device.default_input_config().map_err(|e| e.to_string())?;
    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels() as usize;
    let sample_format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();

    let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let buf2 = buffer.clone();
    let stop2 = stop.clone();

    // cpal's Stream is !Send, so it must be created and owned on its own thread.
    let handle = std::thread::spawn(move || {
        let err_fn = |e| eprintln!("audio stream error: {e}");
        let stream = match sample_format {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut b) = buf2.lock() {
                        for frame in data.chunks(channels) {
                            let m = frame.iter().copied().sum::<f32>() / channels as f32;
                            b.push(m);
                        }
                    }
                },
                err_fn,
                None,
            ),
            _ => {
                eprintln!("unsupported sample format: {sample_format:?}");
                return;
            }
        };
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("failed to build input stream: {e}");
                return;
            }
        };
        if stream.play().is_err() {
            return;
        }
        while !stop2.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        // stream drops here, stopping capture
    });

    Ok(Recorder {
        stop,
        buffer,
        sample_rate,
        handle: Some(handle),
    })
}

pub fn stop_and_take(mut rec: Recorder) -> (Vec<f32>, u32) {
    rec.stop.store(true, Ordering::Relaxed);
    if let Some(h) = rec.handle.take() {
        let _ = h.join();
    }
    let samples = rec.buffer.lock().map(|b| b.clone()).unwrap_or_default();
    (samples, rec.sample_rate)
}

fn resample_to_16k(samples: &[f32], from_rate: u32) -> Vec<f32> {
    let target = 16000u32;
    if from_rate == target || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = target as f32 / from_rate as f32;
    let out_len = (samples.len() as f32 * ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f32 / ratio;
        let idx = src as usize;
        let frac = src - idx as f32;
        let a = samples.get(idx).copied().unwrap_or(0.0);
        let b = samples.get(idx + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }
    out
}

pub fn model_present(model_path: &str) -> bool {
    !model_path.trim().is_empty() && std::path::Path::new(model_path).exists()
}

pub fn transcribe(samples: &[f32], sample_rate: u32, model_path: &str) -> Result<String, String> {
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    if !model_present(model_path) {
        return Err("No Whisper model found. Set a model path in Settings.".into());
    }
    let audio = resample_to_16k(samples, sample_rate);
    if audio.len() < 16000 {
        return Err("Not enough audio captured — try a longer session.".into());
    }

    let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
        .map_err(|e| e.to_string())?;
    let mut state = ctx.create_state().map_err(|e| e.to_string())?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_print_progress(false);
    params.set_print_special(false);
    params.set_print_realtime(false);
    state.full(params, &audio).map_err(|e| e.to_string())?;

    let n = state.full_n_segments().map_err(|e| e.to_string())?;
    let mut text = String::new();
    for i in 0..n {
        if let Ok(seg) = state.full_get_segment_text(i) {
            text.push_str(&seg);
        }
    }
    Ok(text.trim().to_string())
}
