# Audio Engine API

音声キャプチャ・再生エンジンの内部API定義。

---

## 1. 概要

Audio Engineは以下の責務を持つ:

- オーディオデバイスの列挙・選択
- 音声キャプチャ（入力）
- 音声再生（出力）
- ローカルモニタリング
- サンプルレート変換

---

## 2. モジュール構成

```
audio/
├── device.rs       # デバイス管理
├── effects.rs      # エフェクト処理
├── engine.rs       # オーディオエンジン（キャプチャ・再生）
├── error.rs        # エラー型
├── metronome.rs    # メトロノーム
├── plugin.rs       # プラグインホスト
└── recording.rs    # WAV録音
```

---

## 3. デバイス管理 API

### 3.1 デバイス列挙

```rust
/// 利用可能な入力デバイス一覧を取得
///
/// スレッド: 任意
/// ブロッキング: No
fn list_input_devices() -> Result<Vec<AudioDevice>, AudioError>;

/// 利用可能な出力デバイス一覧を取得
///
/// スレッド: 任意
/// ブロッキング: No
fn list_output_devices() -> Result<Vec<AudioDevice>, AudioError>;
```

### 3.2 デバイス情報

```rust
struct AudioDevice {
    /// デバイス識別子
    id: DeviceId,
    /// 表示名
    name: String,
    /// 対応サンプルレート
    supported_sample_rates: Vec<u32>,
    /// 対応チャンネル数
    supported_channels: Vec<u16>,
    /// デフォルトデバイスかどうか
    is_default: bool,
    /// ASIO対応（Windowsのみ）
    is_asio: bool,
}
```

### 3.3 デバイス選択

```rust
/// 入力デバイスを設定
///
/// スレッド: 非リアルタイムスレッドから呼び出すこと
/// ブロッキング: Yes（デバイスオープン完了まで）
///
/// # 注意
/// キャプチャ中に呼び出した場合、キャプチャは停止される
fn set_input_device(device_id: DeviceId) -> Result<(), AudioError>;

/// 出力デバイスを設定
///
/// スレッド: 非リアルタイムスレッドから呼び出すこと
/// ブロッキング: Yes（デバイスオープン完了まで）
///
/// # 注意
/// 再生中に呼び出した場合、再生は停止される
fn set_output_device(device_id: DeviceId) -> Result<(), AudioError>;
```

---

## 4. キャプチャ API

### 4.1 設定

```rust
struct CaptureConfig {
    /// サンプルレート（Hz）
    sample_rate: u32,
    /// チャンネル数
    channels: u16,
    /// フレームサイズ（サンプル数）
    frame_size: u32,
    /// ビット深度
    bit_depth: BitDepth,
}

enum BitDepth {
    I16,
    I24,
    F32,
}
```

### 4.2 キャプチャ開始・停止

```rust
/// 音声キャプチャを開始
///
/// スレッド: 非リアルタイムスレッドから呼び出すこと
/// ブロッキング: No（即座にリターン、キャプチャは別スレッドで実行）
///
/// # コールバック
/// キャプチャされた音声データは `on_audio_captured` コールバックで通知される
fn start_capture(config: CaptureConfig) -> Result<(), AudioError>;

/// 音声キャプチャを停止
///
/// スレッド: 非リアルタイムスレッドから呼び出すこと
/// ブロッキング: Yes（キャプチャスレッド終了まで）
fn stop_capture() -> Result<(), AudioError>;
```

### 4.3 コールバック

```rust
/// 音声キャプチャコールバック
///
/// スレッド: リアルタイムオーディオスレッドから呼び出される
///
/// # 制約
/// - メモリアロケーション禁止
/// - ブロッキングI/O禁止
/// - ミューテックスの長時間保持禁止
/// - 処理時間: frame_size / sample_rate 以内（例: 128/48000 = 2.67ms）
fn on_audio_captured(data: &AudioBuffer, timestamp: u64);
```

---

## 5. 再生 API

### 5.1 設定

```rust
struct PlaybackConfig {
    /// サンプルレート（Hz）
    sample_rate: u32,
    /// チャンネル数
    channels: u16,
    /// フレームサイズ（サンプル数）
    frame_size: u32,
    /// ビット深度
    bit_depth: BitDepth,
}
```

### 5.2 再生開始・停止

```rust
/// 音声再生を開始
///
/// スレッド: 非リアルタイムスレッドから呼び出すこと
/// ブロッキング: No
fn start_playback(config: PlaybackConfig) -> Result<(), AudioError>;

/// 音声再生を停止
///
/// スレッド: 非リアルタイムスレッドから呼び出すこと
/// ブロッキング: Yes
fn stop_playback() -> Result<(), AudioError>;
```

### 5.3 音声データ供給

```rust
/// 再生用音声データをキューに追加
///
/// スレッド: 任意（ロックフリーキュー使用）
/// ブロッキング: No
///
/// # 戻り値
/// キューに追加できた場合は Ok、キューが満杯の場合は Err
fn enqueue_audio(data: AudioBuffer, timestamp: u64) -> Result<(), AudioError>;
```

---

## 6. ローカルモニタリング API

```rust
/// ローカルモニタリングを有効化
///
/// スレッド: 任意
/// ブロッキング: No
///
/// # 動作
/// キャプチャした音声を遅延なしで出力にミックスする
fn enable_local_monitoring() -> Result<(), AudioError>;

/// ローカルモニタリングを無効化
fn disable_local_monitoring() -> Result<(), AudioError>;

/// ローカルモニタリングの音量を設定
///
/// # 引数
/// - volume: 0.0（無音）〜 1.0（最大）
fn set_local_monitoring_volume(volume: f32) -> Result<(), AudioError>;
```

---

## 7. ミキサー API

```rust
/// 参加者の音量を設定
///
/// スレッド: 任意（アトミック操作）
/// ブロッキング: No
///
/// # 引数
/// - participant_id: 参加者ID
/// - volume: 0.0（無音）〜 1.0（最大）
fn set_participant_volume(participant_id: ParticipantId, volume: f32);

/// 参加者をミュート
fn mute_participant(participant_id: ParticipantId);

/// 参加者のミュートを解除
fn unmute_participant(participant_id: ParticipantId);

/// マスター音量を設定
fn set_master_volume(volume: f32);
```

---

## 8. バッファ

### 8.1 AudioBuffer

```rust
struct AudioBuffer {
    /// サンプルデータ（インターリーブ形式）
    data: Vec<f32>,
    /// チャンネル数
    channels: u16,
    /// サンプル数（チャンネルあたり）
    samples: u32,
}
```

### 8.2 リングバッファ

```rust
/// ロックフリーリングバッファ
///
/// 単一プロデューサー・単一コンシューマー（SPSC）
struct RingBuffer<T> {
    // ...
}

impl<T> RingBuffer<T> {
    /// バッファを作成
    fn new(capacity: usize) -> Self;

    /// データをプッシュ（プロデューサー側）
    /// ブロッキング: No
    fn push(&self, item: T) -> Result<(), T>;

    /// データをポップ（コンシューマー側）
    /// ブロッキング: No
    fn pop(&self) -> Option<T>;

    /// 現在のアイテム数
    fn len(&self) -> usize;
}
```

---

## 9. エフェクト API

### 9.1 エフェクトトレイト

```rust
/// エフェクト共通トレイト
pub trait Effect: Send + Sync {
    /// サンプルをインプレース処理
    fn process(&mut self, samples: &mut [f32]);

    /// エフェクト状態をリセット
    fn reset(&mut self);

    /// エフェクト名を取得
    fn name(&self) -> &str;

    /// 有効/無効状態を取得
    fn is_enabled(&self) -> bool;

    /// 有効/無効を設定
    fn set_enabled(&mut self, enabled: bool);
}
```

### 9.2 内蔵エフェクト

```rust
/// 音量調整（リニアゲイン）
pub struct Gain {
    pub gain: f32,  // リニア値（1.0 = unity）
}

impl Gain {
    /// dB値でゲインを作成
    pub fn new(gain_db: f32) -> Self;

    /// dB値でゲインを設定
    pub fn set_gain_db(&mut self, db: f32);
}

/// ローパスフィルタ（1次RC）
pub struct LowPassFilter {
    cutoff: f32,        // カットオフ周波数 (Hz)
    sample_rate: f32,   // サンプルレート
}

impl LowPassFilter {
    pub fn new(cutoff_hz: f32, sample_rate: f32) -> Self;
    pub fn set_cutoff(&mut self, cutoff_hz: f32);
}

/// ハイパスフィルタ（1次RC）
pub struct HighPassFilter {
    cutoff: f32,
    sample_rate: f32,
}

/// コンプレッサー
pub struct Compressor {
    pub threshold_db: f32,  // スレッショルド（dB）
    pub ratio: f32,         // 圧縮比（例: 4.0 = 4:1）
    pub attack_ms: f32,     // アタックタイム（ms）
    pub release_ms: f32,    // リリースタイム（ms）
    pub makeup_db: f32,     // メイクアップゲイン（dB）
}

/// ディレイ
pub struct Delay {
    pub delay_ms: f32,  // ディレイタイム（ms）
    pub feedback: f32,  // フィードバック量（0.0-0.95）
    pub mix: f32,       // ウェット/ドライミックス（0.0=dry, 1.0=wet）
}

/// ノイズゲート
pub struct NoiseGate {
    pub threshold_db: f32,  // スレッショルド（dB）
    pub attack_ms: f32,     // アタックタイム（ms）
    pub release_ms: f32,    // リリースタイム（ms）
}
```

### 9.3 エフェクトチェイン

```rust
/// 複数エフェクトの直列処理
pub struct EffectChain {
    effects: Vec<Box<dyn Effect>>,
    enabled: bool,
}

impl EffectChain {
    pub fn new() -> Self;

    /// エフェクトを追加
    pub fn add(&mut self, effect: Box<dyn Effect>);

    /// エフェクトを削除
    pub fn remove(&mut self, index: usize) -> Option<Box<dyn Effect>>;

    /// サンプルを処理（全エフェクトを順番に適用）
    pub fn process(&mut self, samples: &mut [f32]);

    /// 全エフェクトをリセット
    pub fn reset(&mut self);
}
```

---

## 10. 録音 API

### 10.1 レコーダー

```rust
/// WAV録音
pub struct Recorder {
    sample_rate: u32,
    channels: u16,
    bits_per_sample: u16,  // 16, 24, 32
}

impl Recorder {
    pub fn new(sample_rate: u32, channels: u16, bits_per_sample: u16) -> Self;

    /// 録音開始
    /// スレッド: 非リアルタイム
    pub fn start<P: AsRef<Path>>(&mut self, path: P) -> Result<(), AudioError>;

    /// 録音停止
    /// スレッド: 非リアルタイム
    pub fn stop(&mut self) -> Result<RecordingInfo, AudioError>;

    /// サンプル書き込み
    /// スレッド: 任意（バッファリング済み）
    pub fn write_samples(&mut self, samples: &[f32]) -> Result<(), AudioError>;

    /// 録音中かどうか
    pub fn is_recording(&self) -> bool;

    /// 現在の録音時間（秒）
    pub fn duration_secs(&self) -> f64;
}

/// 録音完了情報
pub struct RecordingInfo {
    pub path: String,
    pub samples: u64,
    pub duration_secs: f64,
    pub file_size: u64,
}
```

### 10.2 WAVフォーマット

| ビット深度 | 形式 | 変換 |
|-----------|------|------|
| 16bit | PCM signed | f32 * 32767 |
| 24bit | PCM signed | f32 * 8388607 |
| 32bit | IEEE float | そのまま |

---

## 11. メトロノーム API

### 11.1 設定

```rust
pub struct MetronomeConfig {
    /// BPM（20-300）
    pub bpm: u32,
    /// 拍子の分子（1小節のビート数）
    pub beats_per_measure: u32,
    /// 拍子の分母（1ビートを表す音符）
    pub beat_value: u32,
    /// クリック音量（0.0-1.0）
    pub volume: f32,
    /// ダウンビートのクリック周波数（Hz）
    pub downbeat_freq: f32,
    /// その他のビートのクリック周波数（Hz）
    pub beat_freq: f32,
}

impl Default for MetronomeConfig {
    fn default() -> Self {
        Self {
            bpm: 120,
            beats_per_measure: 4,
            beat_value: 4,
            volume: 0.5,
            downbeat_freq: 1000.0,
            beat_freq: 800.0,
        }
    }
}
```

### 11.2 メトロノーム状態

```rust
pub struct MetronomeState {
    /// 現在のビート位置（0-indexed）
    pub current_beat: u32,
    /// 現在の小節番号
    pub measure: u32,
    /// 現在のビート内のサンプル位置
    pub sample_position: u64,
    /// 開始からの総サンプル数
    pub total_samples: u64,
}
```

### 11.3 メトロノーム

```rust
pub struct Metronome {
    config: MetronomeConfig,
    sample_rate: u32,
    // ...
}

impl Metronome {
    /// メトロノームを作成
    pub fn new(config: MetronomeConfig, sample_rate: u32) -> Self;

    /// BPM設定（20-300にクランプ）
    pub fn set_bpm(&mut self, bpm: u32);

    /// 現在のBPM取得
    pub fn bpm(&self) -> u32;

    /// 音量設定
    pub fn set_volume(&mut self, volume: f32);

    /// 開始
    pub fn start(&self);

    /// 停止
    pub fn stop(&self);

    /// リセット
    pub fn reset(&self);

    /// 動作中かどうか
    pub fn is_running(&self) -> bool;

    /// 現在の状態取得
    pub fn state(&self) -> MetronomeState;

    /// リモート状態に同期
    pub fn sync_to(&self, state: MetronomeState);

    /// オーディオサンプル生成
    pub fn generate(&self, num_samples: usize) -> Vec<f32>;

    /// 既存バッファにミックス
    pub fn mix_into(&self, buffer: &mut [f32]);
}
```

### 11.4 メトロノーム同期（ネットワーク用）

```rust
pub struct MetronomeSync {
    pub bpm: u32,
    pub beats_per_measure: u32,
    pub current_beat: u32,
    pub measure: u32,
    pub sample_position: u64,
}

impl MetronomeSync {
    /// Metronomeインスタンスから作成
    pub fn from_metronome(metro: &Metronome) -> Self;

    /// バイト列にシリアライズ
    pub fn to_bytes(&self) -> Vec<u8>;

    /// バイト列からデシリアライズ
    pub fn from_bytes(data: &[u8]) -> Option<Self>;
}
```

---

## 12. エラー

```rust
pub enum AudioError {
    /// デバイスが見つからない
    DeviceNotFound(String),
    /// デバイスオープン失敗
    DeviceOpenFailed(String),
    /// サポートされていない設定
    UnsupportedConfig(String),
    /// ストリームエラー
    StreamError(String),
    /// バッファオーバーフロー
    BufferOverflow,
    /// バッファアンダーラン
    BufferUnderrun,
    /// 録音エラー
    RecordingError(String),
    /// プラグインエラー
    PluginError(String),
}
```

---

## 13. スレッドモデル

```
┌─────────────────────────────────────────────────────────┐
│                    Main Thread                          │
│  (デバイス設定、開始/停止)                               │
└─────────────────────────────────────────────────────────┘
                          │
          ┌───────────────┼───────────────┐
          ▼               ▼               ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│ Capture Thread  │ │ Playback Thread │ │ Monitor Thread  │
│ (リアルタイム)   │ │ (リアルタイム)   │ │ (リアルタイム)   │
└─────────────────┘ └─────────────────┘ └─────────────────┘
          │               ▲               │
          │               │               │
          ▼               │               │
┌─────────────────────────┴───────────────┘
│           Lock-free Ring Buffer
└─────────────────────────────────────────┘
```

---

## 14. 使用例

```rust
// デバイス列挙
let inputs = list_input_devices()?;
let outputs = list_output_devices()?;

// デバイス選択
set_input_device(inputs[0].id)?;
set_output_device(outputs[0].id)?;

// キャプチャ設定
let config = CaptureConfig {
    sample_rate: 48000,
    channels: 1,
    frame_size: 128,
    bit_depth: BitDepth::F32,
};

// コールバック設定
set_capture_callback(|data, timestamp| {
    // ネットワーク送信キューに追加
    network.send_audio(data, timestamp);
});

// 開始
start_capture(config)?;
start_playback(playback_config)?;
enable_local_monitoring()?;

// ... セッション中 ...

// 停止
stop_capture()?;
stop_playback()?;
```
