// On-device meeting listener: captures the mic, transcribes locally with Whisper.
// Audio never leaves the machine, and by default only the notes are kept (the
// caller discards the transcript). Requires a Whisper GGML model file that the
// user supplies — nothing is bundled or downloaded automatically.
//
// "Capture all": when enabled, the listener ALSO records system audio (what the
// other participants say, even through headphones) via `ruhear` — ScreenCaptureKit
// on macOS, WASAPI loopback on Windows. Mic + system are downmixed to mono,
// resampled to 16 kHz, and mixed before transcription. This still never leaves the
// machine. On macOS the OS asks for Screen-Recording permission the first time.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

// ScreenCaptureKit / WASAPI-loopback both deliver 48 kHz float samples.
const SYSTEM_AUDIO_RATE: u32 = 48000;

pub struct Recorder {
    stop: Arc<AtomicBool>,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    handle: Option<JoinHandle<()>>,
    // Present only when "capture all" (system audio) is on.
    sys_buffer: Option<Arc<Mutex<Vec<f32>>>>,
    sys_handle: Option<JoinHandle<()>>,
}

#[derive(Default)]
pub struct ListenerState(pub Mutex<Option<Recorder>>);

pub fn start(capture_system: bool) -> Result<Recorder, String> {
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

    // "Capture all": start system-audio capture on its own thread. Best-effort —
    // if it can't start (e.g. permission not yet granted), we degrade to mic-only.
    let (sys_buffer, sys_handle) = if capture_system {
        let (buf, h) = start_system_capture(stop.clone());
        (Some(buf), Some(h))
    } else {
        (None, None)
    };

    Ok(Recorder {
        stop,
        buffer,
        sample_rate,
        handle: Some(handle),
        sys_buffer,
        sys_handle,
    })
}

// Records system output (the other participants) via ruhear on a dedicated thread.
// RUHear owns platform capture resources that aren't Send, so it lives entirely on
// this thread. Channels are downmixed to mono and appended to a shared buffer.
fn start_system_capture(stop: Arc<AtomicBool>) -> (Arc<Mutex<Vec<f32>>>, JoinHandle<()>) {
    let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let buf = buffer.clone();
    let handle = std::thread::spawn(move || {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        // ruhear can panic if system-audio capture is unavailable (e.g. macOS
        // Screen-Recording permission not granted). Contain it so the mic path
        // and the rest of the app keep working.
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let cb_buf = buf.clone();
            let callback: Arc<Mutex<dyn FnMut(ruhear::RUBuffers) + Send>> =
                Arc::new(Mutex::new(move |data: ruhear::RUBuffers| {
                    if data.is_empty() {
                        return;
                    }
                    let channels = data.len();
                    let frames = data.iter().map(|c| c.len()).min().unwrap_or(0);
                    if let Ok(mut b) = cb_buf.lock() {
                        b.reserve(frames);
                        for i in 0..frames {
                            let mut s = 0.0f32;
                            for ch in &data {
                                s += ch[i];
                            }
                            b.push(s / channels as f32);
                        }
                    }
                }));
            let mut ru = ruhear::RUHear::new(callback);
            if let Err(e) = ru.start() {
                eprintln!("system audio capture failed to start: {e}");
                return;
            }
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            let _ = ru.stop();
        }));
        if outcome.is_err() {
            eprintln!(
                "system audio capture unavailable — mic only. \
                 On macOS, grant Screen & System Audio Recording permission and retry."
            );
        }
    });
    (buffer, handle)
}

// Sum two mono streams sample-for-sample, clamping to avoid clipping.
fn mix(a: &[f32], b: &[f32]) -> Vec<f32> {
    let n = a.len().max(b.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let s = a.get(i).copied().unwrap_or(0.0) + b.get(i).copied().unwrap_or(0.0);
        out.push(s.clamp(-1.0, 1.0));
    }
    out
}

pub fn stop_and_take(mut rec: Recorder) -> (Vec<f32>, u32) {
    rec.stop.store(true, Ordering::Relaxed);
    if let Some(h) = rec.handle.take() {
        let _ = h.join();
    }
    if let Some(h) = rec.sys_handle.take() {
        let _ = h.join();
    }
    let mic = rec.buffer.lock().map(|b| b.clone()).unwrap_or_default();

    // Mic-only: return raw samples at their native rate (transcribe resamples).
    let Some(sys_buf) = rec.sys_buffer.as_ref() else {
        return (mic, rec.sample_rate);
    };

    // Capture-all: bring both streams to 16 kHz mono, then mix.
    let sys = sys_buf.lock().map(|b| b.clone()).unwrap_or_default();
    let mic16 = resample_to_16k(&mic, rec.sample_rate);
    let sys16 = resample_to_16k(&sys, SYSTEM_AUDIO_RATE);
    (mix(&mic16, &sys16), 16000)
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
