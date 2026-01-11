<!-- このドキュメントは実装の正です。変更時は実装も同期すること -->

# Network API

P2P音声通信のネットワークAPI定義。

---

## 1. 概要

Network モジュールは以下の責務を持つ:

- P2P接続の確立・維持
- 音声パケットの送受信
- NAT越え（ICE/STUN/TURN）
- Jitterバッファ管理
- FEC処理
- 帯域推定・適応

---

## 2. モジュール構成

```
network/
├── connection.rs       # 接続管理
├── encryption.rs       # 暗号化レイヤー（AES-GCM, X25519）
├── transport.rs        # UDPトランスポート
├── session.rs          # セッション管理
├── signaling.rs        # シグナリング
├── stun.rs             # STUNクライアント
├── fec.rs              # FEC処理
├── jitter_buffer.rs    # Jitterバッファ
├── sequence_tracker.rs # シーケンス追跡
└── error.rs            # ネットワークエラー
```

---

## 3. 接続管理 API

### 3.1 接続状態

```rust
enum ConnectionState {
    /// 切断中
    Disconnected,
    /// 接続試行中
    Connecting,
    /// ICE候補収集中
    GatheringCandidates,
    /// ICE接続中
    CheckingConnectivity,
    /// 接続完了
    Connected,
    /// 再接続中
    Reconnecting,
    /// 失敗
    Failed(ConnectionError),
}
```

### 3.2 接続確立

```rust
/// P2P接続を確立（ICE使用）
///
/// スレッド: 非リアルタイムスレッドから呼び出すこと
/// ブロッキング: No（非同期、状態変化はコールバックで通知）
///
/// # 引数
/// - remote_session: シグナリングで取得したリモートセッション情報
/// - config: 接続設定
async fn connect(
    remote_session: RemoteSession,
    config: ConnectionConfig,
) -> Result<Connection, ConnectionError>;

/// 直接接続（ICEなし、アドバンスオプション）
///
/// スレッド: 非リアルタイムスレッドから呼び出すこと
/// ブロッキング: No
async fn connect_direct(
    remote_addr: SocketAddr,
    config: ConnectionConfig,
) -> Result<Connection, ConnectionError>;
```

### 3.3 接続設定

```rust
struct ConnectionConfig {
    /// STUN サーバー一覧
    stun_servers: Vec<String>,
    /// TURN サーバー（認証情報付き）
    turn_servers: Vec<TurnServer>,
    /// 暗号化を有効にするか
    enable_encryption: bool,
    /// 接続タイムアウト（ms）
    timeout_ms: u32,
    /// 自動再接続を有効にするか
    auto_reconnect: bool,
}

struct TurnServer {
    url: String,
    username: String,
    credential: String,
}
```

### 3.4 切断

```rust
/// 接続を切断
///
/// スレッド: 任意
/// ブロッキング: No
async fn disconnect(&self) -> Result<(), ConnectionError>;
```

---

## 4. 音声送受信 API

### 4.1 送信

```rust
/// 音声データを送信
///
/// スレッド: 任意（ロックフリー）
/// ブロッキング: No
///
/// # 引数
/// - data: 音声データ（エンコード済み）
/// - timestamp: タイムスタンプ（サンプル単位）
///
/// # 動作
/// 1. シーケンス番号を付与
/// 2. FECデータを生成（設定に応じて）
/// 3. パケットを送信キューに追加
fn send_audio(&self, data: &[u8], timestamp: u64) -> Result<(), NetworkError>;
```

### 4.2 受信

```rust
/// 音声データ受信コールバックを設定
///
/// # コールバック引数
/// - participant_id: 送信元参加者ID
/// - data: 音声データ（デコード前）
/// - timestamp: タイムスタンプ
/// - stats: パケット統計
fn set_audio_callback<F>(&self, callback: F)
where
    F: Fn(ParticipantId, &[u8], u64, PacketStats) + Send + 'static;

struct PacketStats {
    /// シーケンス番号
    sequence: u32,
    /// パケットロスがあったか
    had_loss: bool,
    /// FECで復元されたか
    recovered_by_fec: bool,
}
```

---

## 5. Jitterバッファ API

### 5.1 設定

```rust
/// Jitterバッファ設定
#[derive(Debug, Clone)]
pub struct JitterBufferConfig {
    /// 最小バッファ遅延（フレーム数、デフォルト: 1）
    pub min_delay_frames: u32,
    /// 最大バッファ遅延（フレーム数、デフォルト: 10）
    pub max_delay_frames: u32,
    /// 初期バッファ遅延（フレーム数、デフォルト: 2）
    pub initial_delay_frames: u32,
    /// フレーム長（ミリ秒、デフォルト: 2.5）
    pub frame_duration_ms: f32,
}
```

### 5.2 取得結果

```rust
/// Jitterバッファからの取得結果
#[derive(Debug)]
pub enum JitterBufferResult {
    /// パケットあり
    Packet {
        sequence: u32,
        timestamp: u32,
        payload: Vec<u8>,
    },
    /// パケットロス（時間内に受信できず）
    Lost { sequence: u32 },
    /// バッファアンダーラン（パケット不足）
    Underrun,
}
```

### 5.3 JitterBuffer

```rust
/// 適応型Jitterバッファ
///
/// シーケンス番号でインデックス化されたパケットをバッファリングし、
/// ネットワーク状況に応じてバッファサイズを自動調整する。
pub struct JitterBuffer {
    // ... internal fields ...
}

impl JitterBuffer {
    /// デフォルト設定で作成
    pub fn new() -> Self;

    /// カスタム設定で作成
    pub fn with_config(config: JitterBufferConfig) -> Self;

    /// パケットを挿入
    ///
    /// # 引数
    /// - sequence: パケットシーケンス番号
    /// - timestamp: タイムスタンプ（サンプル単位）
    /// - payload: エンコード済み音声データ
    pub fn insert(&mut self, sequence: u32, timestamp: u32, payload: Vec<u8>);

    /// 次のフレームを取得
    ///
    /// シーケンス順でパケットを返す。ロスまたはアンダーランを示す。
    pub fn pop(&mut self) -> JitterBufferResult;

    /// 次のシーケンス番号を確認（削除せず）
    pub fn peek(&self) -> Option<u32>;

    /// 現在のバッファ深度（フレーム数）
    pub fn depth(&self) -> u32;

    /// バッファが空か確認
    pub fn is_empty(&self) -> bool;

    /// 再生開始済みか確認
    pub fn is_playing(&self) -> bool;

    /// 統計情報を取得
    pub fn stats(&self) -> JitterBufferStats;

    /// ネットワーク状況に応じてバッファサイズを適応
    ///
    /// 定期的（例: 100msごと）に呼び出すことでバッファサイズを調整する。
    pub fn adapt(&mut self);

    /// バッファをリセット
    pub fn reset(&mut self);
}
```

### 5.4 統計

```rust
/// Jitterバッファ統計
#[derive(Debug, Clone)]
pub struct JitterBufferStats {
    /// 挿入されたパケット数
    pub packets_inserted: u64,
    /// 再生されたパケット数
    pub packets_played: u64,
    /// ロストパケット数
    pub packets_lost: u64,
    /// 遅延到着パケット数
    pub late_arrivals: u64,
    /// 現在のバッファ深度（フレーム数）
    pub current_depth: u32,
    /// 現在の遅延設定（フレーム数）
    pub current_delay_frames: u32,
    /// ジッター推定値（ms）
    pub jitter_estimate_ms: f32,
}
```

---

## 6. FEC API

### 6.1 設定

```rust
struct FecConfig {
    /// FECを有効にするか
    enabled: bool,
    /// 冗長度（0.0〜1.0、例: 0.1 = 10%）
    redundancy: f32,
    /// グループサイズ（FEC計算単位のパケット数）
    group_size: u32,
}
```

### 6.2 操作

```rust
/// FEC設定を変更
///
/// スレッド: 任意
/// ブロッキング: No
fn configure_fec(&self, config: FecConfig);

/// FEC統計を取得
fn get_fec_stats(&self) -> FecStats;

struct FecStats {
    /// 送信FECパケット数
    fec_packets_sent: u64,
    /// 受信FECパケット数
    fec_packets_received: u64,
    /// FECで復元したパケット数
    packets_recovered: u64,
    /// 復元不可能だったパケット数
    packets_lost: u64,
}
```

---

## 7. シーケンストラッカー API

```rust
/// シーケンス番号追跡によるパケットロス検出
///
/// スライディングウィンドウ方式で順序外パケットと
/// シーケンス番号ラップアラウンドを処理する。
pub struct SequenceTracker {
    /// 最後に受信したシーケンス番号
    last_sequence: Option<u32>,
    /// 最大シーケンス番号
    highest_sequence: u32,
    /// 受信ビットマップ（順序外検出用）
    received_bitmap: u64,
    /// 受信パケット総数
    packets_received: u64,
    /// ロストパケット総数
    packets_lost: u64,
    /// ウィンドウサイズ（デフォルト: 64）
    window_size: u32,
}

impl SequenceTracker {
    /// 新規作成
    pub fn new() -> Self;

    /// パケット受信を記録
    ///
    /// # 戻り値
    /// ロスと判定されたシーケンス番号リスト
    pub fn record(&mut self, sequence: u32) -> Vec<u32>;

    /// パケットロス率を取得（0.0〜1.0）
    pub fn loss_rate(&self) -> f32;

    /// 受信パケット数を取得
    pub fn packets_received(&self) -> u64;

    /// ロストパケット数を取得
    pub fn packets_lost(&self) -> u64;

    /// 最大シーケンス番号を取得
    pub fn highest_sequence(&self) -> u32;

    /// 特定のシーケンス番号が受信済みか確認
    pub fn was_received(&self, sequence: u32) -> bool;

    /// 統計をリセット
    pub fn reset(&mut self);
}
```

---

## 8. 帯域推定 API

```rust
/// 帯域推定設定
struct BandwidthConfig {
    /// モード
    mode: BandwidthMode,
    /// 手動設定時のビットレート（bps）
    manual_bitrate: Option<u32>,
    /// 最小ビットレート（bps）
    min_bitrate: u32,
    /// 最大ビットレート（bps）
    max_bitrate: u32,
}

enum BandwidthMode {
    /// 自動適応
    Auto,
    /// 手動設定
    Manual,
}

/// 帯域推定設定を変更
fn configure_bandwidth(&self, config: BandwidthConfig);

/// 現在の推定帯域を取得
fn get_estimated_bandwidth(&self) -> BandwidthStats;

struct BandwidthStats {
    /// 推定利用可能帯域（bps）
    available_bandwidth: u32,
    /// 現在の送信ビットレート（bps）
    current_bitrate: u32,
    /// 推奨ビットレート（bps）
    recommended_bitrate: u32,
}
```

---

## 9. 接続統計 API

```rust
/// 接続統計を取得
fn get_connection_stats(&self) -> ConnectionStats;

struct ConnectionStats {
    /// RTT（ms）
    rtt_ms: f32,
    /// パケットロス率（0.0〜1.0）
    packet_loss_rate: f32,
    /// ジッター（ms）
    jitter_ms: f32,
    /// 送信バイト数
    bytes_sent: u64,
    /// 受信バイト数
    bytes_received: u64,
    /// 送信パケット数
    packets_sent: u64,
    /// 受信パケット数
    packets_received: u64,
    /// 接続時間（秒）
    uptime_seconds: u64,
}
```

---

## 10. イベント

```rust
enum NetworkEvent {
    /// 接続状態変化
    StateChanged(ConnectionState),
    /// 参加者追加
    ParticipantJoined(ParticipantId),
    /// 参加者退出
    ParticipantLeft(ParticipantId),
    /// 帯域変化
    BandwidthChanged { old: u32, new: u32 },
    /// 接続品質警告
    QualityWarning(QualityWarning),
}

enum QualityWarning {
    /// 高パケットロス
    HighPacketLoss { rate: f32 },
    /// 高レイテンシ
    HighLatency { rtt_ms: f32 },
    /// 帯域不足
    LowBandwidth { available: u32, required: u32 },
}

/// イベントリスナーを設定
fn set_event_listener<F>(&self, listener: F)
where
    F: Fn(NetworkEvent) + Send + 'static;
```

---

## 11. エラー

```rust
enum NetworkError {
    /// 接続タイムアウト
    ConnectionTimeout,
    /// ICE失敗
    IceFailed(String),
    /// DTLS失敗
    DtlsFailed(String),
    /// 切断された
    Disconnected,
    /// 送信バッファ満杯
    SendBufferFull,
    /// 暗号化エラー
    EncryptionError(String),
    /// 鍵交換失敗
    KeyExchangeFailed(String),
    /// 内部エラー
    Internal(String),
}

enum ConnectionError {
    /// タイムアウト
    Timeout,
    /// シグナリング失敗
    SignalingFailed(String),
    /// NAT越え失敗
    NatTraversalFailed,
    /// 認証失敗
    AuthenticationFailed,
    /// 内部エラー
    Internal(String),
}
```

---

## 12. スレッドモデル

```
┌─────────────────────────────────────────────────────────┐
│                    Main Thread                          │
│  (接続設定、開始/停止)                                   │
└─────────────────────────────────────────────────────────┘
                          │
          ┌───────────────┼───────────────┐
          ▼               ▼               ▼
┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐
│  Send Thread    │ │ Receive Thread  │ │  ICE Thread     │
│  (高優先度)      │ │  (高優先度)      │ │  (通常)         │
└─────────────────┘ └─────────────────┘ └─────────────────┘
          │               │
          ▼               ▼
┌─────────────────────────────────────────────────────────┐
│                  Jitter Buffer                          │
│               (ロックフリー実装)                          │
└─────────────────────────────────────────────────────────┘
```

---

## 13. 使用例

```rust
// 接続設定
let config = ConnectionConfig {
    stun_servers: vec!["stun:stun.l.google.com:19302".into()],
    turn_servers: vec![],
    enable_encryption: true,
    timeout_ms: 10000,
    auto_reconnect: true,
};

// 接続
let connection = connect(remote_session, config).await?;

// Jitterバッファ設定
connection.configure_jitter_buffer(JitterBufferConfig {
    min_delay_frames: 1,
    max_delay_frames: 10,
    initial_delay_frames: 4,
    frame_duration_ms: 2.5,
});

// FEC設定
connection.configure_fec(FecConfig {
    enabled: true,
    redundancy: 0.1,
    group_size: 5,
});

// 受信コールバック
connection.set_audio_callback(|participant, data, timestamp, stats| {
    // デコード & 再生キューに追加
    let decoded = decoder.decode(data);
    playback.enqueue_audio(decoded, timestamp);
});

// イベントリスナー
connection.set_event_listener(|event| {
    match event {
        NetworkEvent::QualityWarning(w) => {
            // UI に警告表示
        }
        _ => {}
    }
});

// 送信
connection.send_audio(&encoded_data, timestamp)?;

// 統計取得
let stats = connection.get_connection_stats();
println!("RTT: {}ms, Loss: {:.1}%", stats.rtt_ms, stats.packet_loss_rate * 100.0);

// 切断
connection.disconnect().await?;
```
