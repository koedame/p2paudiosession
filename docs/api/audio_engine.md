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
audio_engine/
├── device.rs       # デバイス管理
├── capture.rs      # 音声キャプチャ
├── playback.rs     # 音声再生
├── monitor.rs      # ローカルモニタリング
├── resampler.rs    # リサンプリング
└── buffer.rs       # リングバッファ
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

## 9. エラー

```rust
enum AudioError {
    /// デバイスが見つからない
    DeviceNotFound,
    /// デバイスオープン失敗
    DeviceOpenFailed(String),
    /// サポートされていない設定
    UnsupportedConfig,
    /// バッファオーバーフロー
    BufferOverflow,
    /// バッファアンダーラン
    BufferUnderrun,
    /// 内部エラー
    Internal(String),
}
```

---

## 10. スレッドモデル

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

## 11. 使用例

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
