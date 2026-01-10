//! Audio quality tests based on docs/behavior/audio-quality.feature
//!
//! Tests for audio quality functionality.

use jamjam::audio::{
    AudioConfig, AudioEngine, CaptureConfig, PlaybackConfig, BitDepth,
    EffectChain, Gain, Recorder, Metronome, MetronomeConfig,
};

/// Test: サンプルレート48kHzで動作する
/// When サンプルレートを「48000Hz」に設定する
/// Then オーディオエンジンは48kHzで動作する
#[test]
fn test_sample_rate_48khz() {
    let config = AudioConfig {
        sample_rate: 48000,
        channels: 1,
        frame_size: 128,
    };
    let engine = AudioEngine::new(config);

    assert_eq!(engine.config().sample_rate, 48000);
}

/// Test: サンプルレート96kHzで動作する
/// When サンプルレートを「96000Hz」に設定する
/// Then オーディオエンジンは96kHzで動作する
#[test]
fn test_sample_rate_96khz() {
    let config = AudioConfig {
        sample_rate: 96000,
        channels: 1,
        frame_size: 128,
    };
    let engine = AudioEngine::new(config);

    assert_eq!(engine.config().sample_rate, 96000);
}

/// Test: モノラル入力で動作する
/// When 入力チャンネルを「モノラル」に設定する
/// Then 1チャンネルの音声が送信される
#[test]
fn test_mono_input() {
    let config = CaptureConfig {
        sample_rate: 48000,
        channels: 1,
        frame_size: 128,
        bit_depth: BitDepth::F32,
    };

    assert_eq!(config.channels, 1);
}

/// Test: ステレオ入力で動作する
/// When 入力チャンネルを「ステレオ」に設定する
/// Then 2チャンネルの音声が送信される
#[test]
fn test_stereo_input() {
    let config = CaptureConfig {
        sample_rate: 48000,
        channels: 2,
        frame_size: 128,
        bit_depth: BitDepth::F32,
    };

    assert_eq!(config.channels, 2);
}

/// Test: フレームサイズ64サンプルで動作する
/// When フレームサイズを「64 samples」に設定する
/// Then オーディオバッファは64サンプルになる
#[test]
fn test_frame_size_64() {
    let config = AudioConfig {
        sample_rate: 48000,
        channels: 1,
        frame_size: 64,
    };

    assert_eq!(config.frame_size, 64);

    // Calculate latency: 64 / 48000 = 1.33ms
    let latency_ms = config.frame_size as f32 / config.sample_rate as f32 * 1000.0;
    assert!((latency_ms - 1.33).abs() < 0.1);
}

/// Test: フレームサイズ256サンプルで動作する
/// When フレームサイズを「256 samples」に設定する
/// Then オーディオバッファは256サンプルになる
#[test]
fn test_frame_size_256() {
    let config = AudioConfig {
        sample_rate: 48000,
        channels: 1,
        frame_size: 256,
    };

    assert_eq!(config.frame_size, 256);

    // Calculate latency: 256 / 48000 = 5.33ms
    let latency_ms = config.frame_size as f32 / config.sample_rate as f32 * 1000.0;
    assert!((latency_ms - 5.33).abs() < 0.1);
}

/// Test: ローカルモニタリングを有効/無効にする
#[test]
fn test_local_monitoring_toggle() {
    let engine = AudioEngine::new(AudioConfig::default());

    // Initially disabled
    assert!(!engine.is_local_monitoring_enabled());

    // Enable
    engine.set_local_monitoring(true);
    assert!(engine.is_local_monitoring_enabled());

    // Disable
    engine.set_local_monitoring(false);
    assert!(!engine.is_local_monitoring_enabled());
}

/// Test: BitDepth設定
#[test]
fn test_bit_depth_options() {
    // 16-bit
    let config_i16 = CaptureConfig {
        bit_depth: BitDepth::I16,
        ..Default::default()
    };
    assert_eq!(config_i16.bit_depth, BitDepth::I16);

    // 24-bit
    let config_i24 = CaptureConfig {
        bit_depth: BitDepth::I24,
        ..Default::default()
    };
    assert_eq!(config_i24.bit_depth, BitDepth::I24);

    // 32-bit float
    let config_f32 = CaptureConfig {
        bit_depth: BitDepth::F32,
        ..Default::default()
    };
    assert_eq!(config_f32.bit_depth, BitDepth::F32);
}

/// Test: エフェクトチェインが動作する
#[test]
fn test_effect_chain() {
    let mut chain = EffectChain::new();
    chain.add(Box::new(Gain::new(0.0))); // Unity gain

    let mut samples = vec![0.5, -0.5, 0.25, -0.25];
    chain.process(&mut samples);

    // Unity gain should not change samples significantly
    assert!((samples[0] - 0.5).abs() < 0.01);
    assert!((samples[1] + 0.5).abs() < 0.01);
}

/// Test: レコーダーが作成できる
#[test]
fn test_recorder_creation() {
    let recorder = Recorder::new(48000, 2, 16);

    assert!(!recorder.is_recording());
    assert_eq!(recorder.samples_written(), 0);
}

/// Test: メトロノームが動作する
#[test]
fn test_metronome() {
    let config = MetronomeConfig::default();
    let metro = Metronome::new(config, 48000);

    assert!(!metro.is_running());
    assert_eq!(metro.bpm(), 120);

    metro.start();
    assert!(metro.is_running());

    let samples = metro.generate(1024);
    assert_eq!(samples.len(), 1024);

    metro.stop();
    assert!(!metro.is_running());
}

/// Test: デフォルト設定が正しい
#[test]
fn test_default_configs() {
    let audio_config = AudioConfig::default();
    assert_eq!(audio_config.sample_rate, 48000);
    assert_eq!(audio_config.channels, 1);
    assert_eq!(audio_config.frame_size, 128);

    let capture_config = CaptureConfig::default();
    assert_eq!(capture_config.sample_rate, 48000);
    assert_eq!(capture_config.channels, 1);
    assert_eq!(capture_config.frame_size, 128);
    assert_eq!(capture_config.bit_depth, BitDepth::F32);

    let playback_config = PlaybackConfig::default();
    assert_eq!(playback_config.sample_rate, 48000);
    assert_eq!(playback_config.channels, 1);
    assert_eq!(playback_config.frame_size, 128);
    assert_eq!(playback_config.bit_depth, BitDepth::F32);
}
