# Plugin API

オーディオプラグインホストのAPI定義。

---

## 1. 概要

Pluginモジュールは以下の責務を持つ:

- プラグインの検索・列挙
- プラグインのロード・アンロード
- プラグインへの音声処理委譲
- プラグインパラメータの操作
- プラグインエディタ（UI）の表示

---

## 2. 対応フォーマット

| フォーマット | 拡張子 | 対応状況 |
|-------------|--------|---------|
| VST3 | .vst3 | インターフェース実装済 |
| CLAP | .clap | インターフェース実装済 |
| AudioUnit | .component | インターフェース実装済（macOSのみ） |

---

## 3. プラグイン情報

```rust
/// プラグインフォーマット種別
pub enum PluginFormat {
    Vst3,
    Clap,
    AudioUnit,
}

/// プラグイン情報
pub struct PluginInfo {
    /// プラグイン名
    pub name: String,
    /// ベンダー名
    pub vendor: String,
    /// バージョン
    pub version: String,
    /// フォーマット
    pub format: PluginFormat,
    /// ファイルパス
    pub path: String,
    /// ユニークID
    pub uid: String,
    /// 入力チャンネル数
    pub num_inputs: u32,
    /// 出力チャンネル数
    pub num_outputs: u32,
    /// エディタUIを持つか
    pub has_editor: bool,
}
```

---

## 4. プラグインパラメータ

```rust
/// プラグインパラメータ
pub struct PluginParameter {
    /// パラメータID
    pub id: u32,
    /// パラメータ名
    pub name: String,
    /// 現在値
    pub value: f32,
    /// 最小値
    pub min: f32,
    /// 最大値
    pub max: f32,
    /// デフォルト値
    pub default: f32,
    /// 単位（dB, Hz, % など）
    pub unit: String,
}
```

---

## 5. プラグイントレイト

```rust
/// オーディオプラグインインターフェース
pub trait AudioPlugin: Send {
    /// プラグイン情報を取得
    fn info(&self) -> &PluginInfo;

    /// プラグインを初期化
    ///
    /// スレッド: 非リアルタイム
    /// ブロッキング: Yes
    fn initialize(
        &mut self,
        sample_rate: f64,
        max_block_size: u32,
    ) -> Result<(), AudioError>;

    /// プラグインをアクティベート
    ///
    /// スレッド: 非リアルタイム
    fn activate(&mut self) -> Result<(), AudioError>;

    /// プラグインをディアクティベート
    fn deactivate(&mut self);

    /// 音声を処理
    ///
    /// スレッド: リアルタイム
    /// 制約: メモリアロケーション禁止
    fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]);

    /// パラメータ数を取得
    fn num_parameters(&self) -> usize;

    /// パラメータ情報を取得
    fn parameter(&self, index: usize) -> Option<PluginParameter>;

    /// パラメータ値を設定
    fn set_parameter(&mut self, index: usize, value: f32);

    /// パラメータ値を取得
    fn get_parameter(&self, index: usize) -> f32;

    /// エディタウィンドウを開く
    ///
    /// # 引数
    /// - parent: 親ウィンドウハンドル
    fn open_editor(&mut self, parent: *mut std::ffi::c_void) -> Result<(), AudioError>;

    /// エディタウィンドウを閉じる
    fn close_editor(&mut self);

    /// エディタが開いているか
    fn is_editor_open(&self) -> bool;
}
```

---

## 6. プラグインスキャナー

```rust
/// プラグイン検索
pub struct PluginScanner {
    search_paths: Vec<String>,
}

impl PluginScanner {
    /// デフォルト検索パスでスキャナーを作成
    pub fn new() -> Self;

    /// カスタム検索パスを追加
    pub fn add_search_path(&mut self, path: &str);

    /// プラグインをスキャン
    ///
    /// スレッド: 非リアルタイム
    /// ブロッキング: Yes（ディスクI/O）
    pub fn scan(&self) -> Vec<PluginInfo>;
}
```

### 6.1 デフォルト検索パス

| プラットフォーム | VST3パス | CLAPパス |
|-----------------|----------|----------|
| Linux | ~/.vst3, /usr/lib/vst3, /usr/local/lib/vst3 | ~/.clap, /usr/lib/clap, /usr/local/lib/clap |
| macOS | ~/Library/Audio/Plug-Ins/VST3, /Library/Audio/Plug-Ins/VST3 | ~/Library/Audio/Plug-Ins/CLAP, /Library/Audio/Plug-Ins/CLAP |
| Windows | C:\Program Files\Common Files\VST3 | C:\Program Files\Common Files\CLAP |

---

## 7. プラグインホスト

```rust
/// プラグインホスト（複数プラグイン管理）
pub struct PluginHost {
    plugins: Vec<Box<dyn AudioPlugin>>,
    sample_rate: f64,
    block_size: u32,
}

impl PluginHost {
    /// ホストを作成
    pub fn new(sample_rate: f64, block_size: u32) -> Self;

    /// プラグインをロード
    ///
    /// スレッド: 非リアルタイム
    /// 戻り値: プラグインインデックス
    pub fn load_plugin(&mut self, path: &str) -> Result<usize, AudioError>;

    /// プラグインをアンロード
    pub fn unload_plugin(&mut self, index: usize) -> Result<(), AudioError>;

    /// 全プラグインで音声を処理
    ///
    /// スレッド: リアルタイム
    pub fn process(&mut self, inputs: &[&[f32]], outputs: &mut [&mut [f32]]);

    /// ロード済みプラグイン数
    pub fn num_plugins(&self) -> usize;

    /// プラグイン情報を取得
    pub fn plugin_info(&self, index: usize) -> Option<&PluginInfo>;
}
```

---

## 8. エラー

```rust
pub enum AudioError {
    /// プラグインエラー
    PluginError(String),
    // ...
}
```

プラグイン関連のエラー例:
- プラグインファイルが見つからない
- プラグインのロードに失敗
- プラグインの初期化に失敗
- プラグインフォーマットが不正

---

## 9. 使用例

```rust
// スキャナーでプラグインを検索
let scanner = PluginScanner::new();
let available = scanner.scan();

for info in &available {
    println!("{} by {} ({})", info.name, info.vendor, info.format);
}

// ホストを作成
let mut host = PluginHost::new(48000.0, 512);

// プラグインをロード
let index = host.load_plugin("/path/to/plugin.vst3")?;

// パラメータを取得・設定
if let Some(info) = host.plugin_info(index) {
    // プラグイン情報を表示
}

// 音声処理
let inputs: Vec<&[f32]> = vec![&input_left, &input_right];
let mut outputs: Vec<&mut [f32]> = vec![&mut output_left, &mut output_right];
host.process(&inputs, &mut outputs);

// アンロード
host.unload_plugin(index)?;
```

---

## 10. 制限事項

現在の実装状況:

| 機能 | 状態 |
|------|------|
| プラグインスキャン | 実装済（ファイル検出のみ） |
| プラグインロード | 未実装（SDKインテグレーション必要） |
| 音声処理 | インターフェース定義済 |
| パラメータ操作 | インターフェース定義済 |
| エディタUI | インターフェース定義済 |

完全なプラグインロード機能には、各フォーマットのSDK（VST3 SDK、CLAP SDK）のインテグレーションが必要。
