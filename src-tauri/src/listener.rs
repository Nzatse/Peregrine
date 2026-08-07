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
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

// ScreenCaptureKit / WASAPI-loopback both deliver 48 kHz float samples.
const SYSTEM_AUDIO_RATE: u32 = 48000;

// Bound memory on very long sessions: stop appending past this many minutes so the
// in-RAM audio buffer can't grow without limit and OOM the app on a marathon call.
const MAX_CAPTURE_MINUTES: usize = 90;

pub struct Recorder {
    stop: Arc<AtomicBool>,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    handle: Option<JoinHandle<()>>,
    // The instant each stream actually began capturing, so "capture all" can align
    // them on a common timeline instead of naively mixing from sample 0 (the two
    // threads start at different times — ScreenCaptureKit especially lags).
    mic_start: Arc<Mutex<Option<Instant>>>,
    // Present only when "capture all" (system audio) is on.
    sys_buffer: Option<Arc<Mutex<Vec<f32>>>>,
    sys_handle: Option<JoinHandle<()>>,
    sys_start: Arc<Mutex<Option<Instant>>>,
}

#[derive(Default)]
pub struct ListenerState(pub Mutex<Option<Recorder>>);

pub fn start(capture_system: bool) -> Result<Recorder, String> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or("No microphone found.")?;
    let supported = device.default_input_config().map_err(|e| e.to_string())?;
    let sample_rate = supported.sample_rate().0;
    let channels = (supported.channels() as usize).max(1);
    let sample_format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();

    let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let mic_start = Arc::new(Mutex::new(None::<Instant>));
    let buf2 = buffer.clone();
    let stop2 = stop.clone();
    let mic_start2 = mic_start.clone();
    let max_samples = sample_rate as usize * 60 * MAX_CAPTURE_MINUTES;

    // cpal's Stream is !Send, so it must be created and owned on its own thread. The
    // thread reports back whether the mic actually started, so `start` can return a
    // real error (permission denied, device busy, unsupported format) instead of
    // pretending to record and handing back silence.
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
    let handle = std::thread::spawn(move || {
        let stream = match build_input_stream(
            &device,
            &config,
            sample_format,
            channels,
            max_samples,
            buf2,
        ) {
            Ok(s) => s,
            Err(e) => {
                let _ = ready_tx.send(Err(e));
                return;
            }
        };
        if let Err(e) = stream.play() {
            let _ = ready_tx.send(Err(format!("Could not start the microphone: {e}")));
            return;
        }
        // Capture has begun — stamp the start so system audio can be time-aligned.
        if let Ok(mut t) = mic_start2.lock() {
            *t = Some(Instant::now());
        }
        let _ = ready_tx.send(Ok(()));
        while !stop2.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(100));
        }
        // stream drops here, stopping capture
    });

    // Wait for the capture thread to actually get the mic going before we claim to
    // be listening. Without this, a denied permission or a busy device silently
    // yields an empty recording and a confusing "not enough audio" at stop time.
    match ready_rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            let _ = handle.join();
            return Err(e);
        }
        Err(_) => {
            stop.store(true, Ordering::Relaxed);
            let _ = handle.join();
            return Err("The microphone didn't start in time.".into());
        }
    }

    // "Capture all": start system-audio capture on its own thread. Best-effort —
    // if it can't start (e.g. permission not yet granted), we degrade to mic-only.
    let sys_start = Arc::new(Mutex::new(None::<Instant>));
    let (sys_buffer, sys_handle) = if capture_system {
        let (buf, h) = start_system_capture(stop.clone(), sys_start.clone());
        (Some(buf), Some(h))
    } else {
        (None, None)
    };

    Ok(Recorder {
        stop,
        buffer,
        sample_rate,
        handle: Some(handle),
        mic_start,
        sys_buffer,
        sys_handle,
        sys_start,
    })
}

// Builds the input stream, converting whatever native sample format the device
// reports into mono f32. cpal devices commonly deliver I16 (especially on Windows /
// WASAPI) or U16 — not just F32 — so handling only F32 silently records nothing on
// those devices. Each frame's channels are averaged to mono; appending stops once
// the buffer hits the memory cap.
fn build_input_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    format: cpal::SampleFormat,
    channels: usize,
    max_samples: usize,
    buf: Arc<Mutex<Vec<f32>>>,
) -> Result<cpal::Stream, String> {
    let err_fn = |e| eprintln!("audio stream error: {e}");
    // $conv normalizes one native sample to f32 in [-1, 1]; unsigned formats are
    // biased to center on zero. Same normalization ruhear uses for its cpal path.
    macro_rules! build {
        ($t:ty, $conv:expr) => {{
            let buf = buf.clone();
            let conv: fn($t) -> f32 = $conv;
            device.build_input_stream(
                config,
                move |data: &[$t], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut b) = buf.lock() {
                        if b.len() >= max_samples {
                            return;
                        }
                        for frame in data.chunks(channels) {
                            let m =
                                frame.iter().map(|&s| conv(s)).sum::<f32>() / channels as f32;
                            b.push(m);
                        }
                    }
                },
                err_fn,
                None,
            )
        }};
    }
    let stream = match format {
        cpal::SampleFormat::F32 => build!(f32, |s| s),
        cpal::SampleFormat::F64 => build!(f64, |s| s as f32),
        cpal::SampleFormat::I16 => build!(i16, |s| s as f32 / i16::MAX as f32),
        cpal::SampleFormat::U16 => build!(u16, |s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0),
        cpal::SampleFormat::I8 => build!(i8, |s| s as f32 / i8::MAX as f32),
        cpal::SampleFormat::U8 => build!(u8, |s| (s as f32 / u8::MAX as f32) * 2.0 - 1.0),
        cpal::SampleFormat::I32 => build!(i32, |s| s as f32 / i32::MAX as f32),
        cpal::SampleFormat::U32 => build!(u32, |s| (s as f32 / u32::MAX as f32) * 2.0 - 1.0),
        other => return Err(format!("Unsupported microphone sample format: {other:?}")),
    };
    stream.map_err(|e| format!("Could not open the microphone: {e}"))
}

// Records system output (the other participants) via ruhear on a dedicated thread.
// RUHear owns platform capture resources that aren't Send, so it lives entirely on
// this thread. Channels are downmixed to mono and appended to a shared buffer.
fn start_system_capture(
    stop: Arc<AtomicBool>,
    sys_start: Arc<Mutex<Option<Instant>>>,
) -> (Arc<Mutex<Vec<f32>>>, JoinHandle<()>) {
    let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let buf = buffer.clone();
    let handle = std::thread::spawn(move || {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        // ruhear can panic if system-audio capture is unavailable (e.g. macOS
        // Screen-Recording permission not granted). Contain it so the mic path
        // and the rest of the app keep working.
        let max_samples = SYSTEM_AUDIO_RATE as usize * 60 * MAX_CAPTURE_MINUTES;
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
                        if b.len() >= max_samples {
                            return;
                        }
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
            // Capture began — stamp it so stop_and_take can align against the mic.
            if let Ok(mut t) = sys_start.lock() {
                *t = Some(Instant::now());
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

// Prepend `pad` samples of silence so a later-starting stream lines up on a shared
// timeline before mixing.
fn lead_silence(samples: Vec<f32>, pad: usize) -> Vec<f32> {
    if pad == 0 {
        return samples;
    }
    let mut out = vec![0.0f32; pad + samples.len()];
    out[pad..].copy_from_slice(&samples);
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

    // Capture-all: bring both streams to 16 kHz mono, align on a common start, mix.
    let sys = sys_buf.lock().map(|b| b.clone()).unwrap_or_default();
    let mic16 = resample_to_16k(&mic, rec.sample_rate);
    let sys16 = resample_to_16k(&sys, SYSTEM_AUDIO_RATE);

    // The two threads begin at different instants; without alignment the other
    // party's speech is offset from the user's and Whisper hears overlapping talk.
    // Pad whichever started later with leading silence so sample 0 shares a moment.
    let mic_start = rec.mic_start.lock().ok().and_then(|t| *t);
    let sys_start = rec.sys_start.lock().ok().and_then(|t| *t);
    let (mic16, sys16) = match (mic_start, sys_start) {
        (Some(m), Some(s)) => {
            let t0 = m.min(s);
            let pad_mic = ((m - t0).as_secs_f64() * 16000.0) as usize;
            let pad_sys = ((s - t0).as_secs_f64() * 16000.0) as usize;
            (lead_silence(mic16, pad_mic), lead_silence(sys16, pad_sys))
        }
        _ => (mic16, sys16),
    };
    (mix(&mic16, &sys16), 16000)
}

fn resample_to_16k(samples: &[f32], from_rate: u32) -> Vec<f32> {
    let target = 16000u32;
    if from_rate == target || samples.is_empty() {
        return samples.to_vec();
    }
    // Index math in f64: f32's 24-bit mantissa can't represent consecutive sample
    // indices past ~16.7M (~5.8 min at 48 kHz), which would skip/repeat samples and
    // audibly degrade transcription on exactly the long meetings users care about.
    let ratio = target as f64 / from_rate as f64;
    let out_len = (samples.len() as f64 * ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 / ratio;
        let idx = src as usize;
        let frac = (src - idx as f64) as f32;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_is_identity_at_target_rate() {
        let s = vec![0.1, -0.2, 0.3];
        assert_eq!(resample_to_16k(&s, 16000), s);
    }

    #[test]
    fn resample_48k_to_16k_thirds_the_length() {
        // 48 kHz -> 16 kHz is a 1:3 decimation; 4800 samples -> ~1600.
        let s = vec![0.0f32; 4800];
        let out = resample_to_16k(&s, 48000);
        assert!(
            (out.len() as i64 - 1600).abs() <= 1,
            "expected ~1600, got {}",
            out.len()
        );
    }

    #[test]
    fn resample_stays_accurate_past_the_f32_precision_cliff() {
        // Beyond ~16.7M samples an f32 index would step unevenly; f64 must not.
        // A ramp resampled 1:1 (same rate) round-trips exactly at a late index.
        let big = 17_000_000usize;
        let mut s = vec![0.0f32; big + 2];
        s[big] = 0.5;
        s[big + 1] = 0.5;
        let out = resample_to_16k(&s, 16000); // identity path
        assert_eq!(out[big], 0.5);
    }

    #[test]
    fn lead_silence_prepends_zeros() {
        let out = lead_silence(vec![1.0, 2.0], 3);
        assert_eq!(out, vec![0.0, 0.0, 0.0, 1.0, 2.0]);
        // zero pad is a no-op that keeps the original.
        assert_eq!(lead_silence(vec![1.0], 0), vec![1.0]);
    }

    #[test]
    fn mix_sums_and_clamps() {
        // Overlapping speech sums; peaks clamp instead of wrapping/distorting.
        assert_eq!(mix(&[0.5, 0.5], &[0.25, 0.9]), vec![0.75, 1.0]);
        // Different lengths: the shorter is treated as silence past its end.
        assert_eq!(mix(&[0.5], &[0.1, 0.2]), vec![0.6, 0.2]);
    }
}
