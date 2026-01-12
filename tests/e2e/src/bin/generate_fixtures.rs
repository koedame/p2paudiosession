//! Generate reference audio fixtures for E2E testing
//!
//! Usage: cargo run --bin generate_fixtures

use hound::{WavSpec, WavWriter};
use std::f32::consts::PI;
use std::path::Path;

const SAMPLE_RATE: u32 = 48000;

fn main() {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/reference_audio");

    std::fs::create_dir_all(&fixtures_dir).expect("Failed to create fixtures directory");

    println!("Generating audio fixtures in {:?}", fixtures_dir);

    // Generate various test audio files
    generate_sine_wave(&fixtures_dir.join("sine_440hz_1s.wav"), 440.0, 1.0);
    generate_sine_wave(&fixtures_dir.join("sine_1000hz_1s.wav"), 1000.0, 1.0);
    generate_sine_wave(&fixtures_dir.join("sine_440hz_5s.wav"), 440.0, 5.0);

    generate_sweep(&fixtures_dir.join("sweep_100_10000hz_2s.wav"), 100.0, 10000.0, 2.0);
    generate_sweep(&fixtures_dir.join("sweep_20_20000hz_5s.wav"), 20.0, 20000.0, 5.0);

    generate_silence(&fixtures_dir.join("silence_1s.wav"), 1.0);

    generate_white_noise(&fixtures_dir.join("white_noise_1s.wav"), 1.0, 0.1);

    generate_impulse(&fixtures_dir.join("impulse.wav"));

    generate_speech_like(&fixtures_dir.join("speech_like_3s.wav"), 3.0);

    println!("Done! Generated {} audio files", 9);
}

fn create_wav_writer(path: &Path) -> WavWriter<std::io::BufWriter<std::fs::File>> {
    let spec = WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    WavWriter::create(path, spec).expect("Failed to create WAV file")
}

fn generate_sine_wave(path: &Path, frequency: f32, duration_sec: f32) {
    println!("  Generating sine wave: {:?}", path.file_name().unwrap());

    let mut writer = create_wav_writer(path);
    let num_samples = (SAMPLE_RATE as f32 * duration_sec) as usize;

    for i in 0..num_samples {
        let t = i as f32 / SAMPLE_RATE as f32;
        let sample = (2.0 * PI * frequency * t).sin() * 0.5;

        // Write to both channels
        writer.write_sample(sample).unwrap();
        writer.write_sample(sample).unwrap();
    }

    writer.finalize().unwrap();
}

fn generate_sweep(path: &Path, start_freq: f32, end_freq: f32, duration_sec: f32) {
    println!("  Generating frequency sweep: {:?}", path.file_name().unwrap());

    let mut writer = create_wav_writer(path);
    let num_samples = (SAMPLE_RATE as f32 * duration_sec) as usize;

    let mut phase = 0.0f32;

    for i in 0..num_samples {
        let t = i as f32 / num_samples as f32;
        // Logarithmic sweep
        let freq = start_freq * (end_freq / start_freq).powf(t);

        phase += 2.0 * PI * freq / SAMPLE_RATE as f32;
        if phase > 2.0 * PI {
            phase -= 2.0 * PI;
        }

        let sample = phase.sin() * 0.5;

        writer.write_sample(sample).unwrap();
        writer.write_sample(sample).unwrap();
    }

    writer.finalize().unwrap();
}

fn generate_silence(path: &Path, duration_sec: f32) {
    println!("  Generating silence: {:?}", path.file_name().unwrap());

    let mut writer = create_wav_writer(path);
    let num_samples = (SAMPLE_RATE as f32 * duration_sec) as usize;

    for _ in 0..num_samples {
        writer.write_sample(0.0f32).unwrap();
        writer.write_sample(0.0f32).unwrap();
    }

    writer.finalize().unwrap();
}

fn generate_white_noise(path: &Path, duration_sec: f32, amplitude: f32) {
    println!("  Generating white noise: {:?}", path.file_name().unwrap());

    let mut writer = create_wav_writer(path);
    let num_samples = (SAMPLE_RATE as f32 * duration_sec) as usize;

    // Simple PRNG for reproducible noise
    let mut state = 12345u64;

    for _ in 0..num_samples {
        // LCG random number generator
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        let random = ((state >> 33) as f32 / u32::MAX as f32) * 2.0 - 1.0;
        let sample = random * amplitude;

        writer.write_sample(sample).unwrap();
        writer.write_sample(sample).unwrap();
    }

    writer.finalize().unwrap();
}

fn generate_impulse(path: &Path) {
    println!("  Generating impulse: {:?}", path.file_name().unwrap());

    let mut writer = create_wav_writer(path);

    // 100ms of silence, impulse, 100ms of silence
    let silence_samples = (SAMPLE_RATE as f32 * 0.1) as usize;

    // Leading silence
    for _ in 0..silence_samples {
        writer.write_sample(0.0f32).unwrap();
        writer.write_sample(0.0f32).unwrap();
    }

    // Impulse (single sample at full amplitude)
    writer.write_sample(1.0f32).unwrap();
    writer.write_sample(1.0f32).unwrap();

    // Trailing silence
    for _ in 0..silence_samples {
        writer.write_sample(0.0f32).unwrap();
        writer.write_sample(0.0f32).unwrap();
    }

    writer.finalize().unwrap();
}

fn generate_speech_like(path: &Path, duration_sec: f32) {
    println!("  Generating speech-like audio: {:?}", path.file_name().unwrap());

    let mut writer = create_wav_writer(path);
    let num_samples = (SAMPLE_RATE as f32 * duration_sec) as usize;

    // Simulate speech with fundamental frequency modulation and formants
    let f0_base = 150.0; // Fundamental frequency

    for i in 0..num_samples {
        let t = i as f32 / SAMPLE_RATE as f32;

        // Modulate fundamental frequency (simulating pitch variation)
        let f0 = f0_base * (1.0 + 0.1 * (2.0 * PI * 0.5 * t).sin());

        // Generate harmonics
        let mut sample = 0.0f32;

        // Fundamental
        sample += (2.0 * PI * f0 * t).sin() * 0.3;

        // Harmonics with decreasing amplitude
        for h in 2..=8 {
            let harmonic_amp = 0.3 / (h as f32);
            sample += (2.0 * PI * f0 * h as f32 * t).sin() * harmonic_amp;
        }

        // Add some amplitude envelope (syllable-like)
        let envelope = ((2.0 * PI * 3.0 * t).sin() * 0.5 + 0.5).max(0.1);
        sample *= envelope;

        // Limit amplitude
        sample = sample.clamp(-0.8, 0.8);

        writer.write_sample(sample).unwrap();
        writer.write_sample(sample).unwrap();
    }

    writer.finalize().unwrap();
}
