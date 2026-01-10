# jamjam

ミュージシャン向け超低レイテンシP2P音声通信アプリ

## 概要

jamjamは、ミュージシャンがインターネット越しにリアルタイムでジャムセッションや遠隔レコーディングを行うためのアプリケーションである。

## 実装状況

| フェーズ | 機能 | 状態 |
|---------|------|------|
| Phase 1 | コア機能（オーディオエンジン、UDPトランスポート、プロトコル、CLI） | 完了 |
| Phase 2 | ネットワーク拡張（STUN、シグナリング、マルチピア、FEC） | 完了 |
| Phase 3 | Tauri GUI（デスクトップUI、ミキサー、設定画面） | 完了 |
| Phase 4 | 録音機能、メトロノーム共有 | 完了 |
| Phase 5 | エフェクト、VST/CLAPプラグインホスト | 完了 |

## ドキュメント構成

| ドキュメント | 説明 |
|-------------|------|
| [architecture.md](./architecture.md) | 技術構成（最重要） |
| [adr/](./adr/) | 設計判断記録 |
| [behavior/](./behavior/) | 振る舞い定義（BDD） |
| [api/](./api/) | API境界定義 |

### API仕様

| ドキュメント | 説明 |
|-------------|------|
| [api/audio_engine.md](./api/audio_engine.md) | オーディオエンジンAPI |
| [api/network.md](./api/network.md) | ネットワークAPI |
| [api/signaling.md](./api/signaling.md) | シグナリングAPI |
| [api/plugin.md](./api/plugin.md) | プラグインホストAPI |
| [api/i18n.md](./api/i18n.md) | 国際化API |

## 対象ユースケース

- オンラインジャムセッション
- 遠隔レコーディング
- リアルタイム演奏セッション

## 対象プラットフォーム

| プラットフォーム | 状態 |
|-----------------|------|
| Windows | 対応済 |
| macOS | 対応済 |
| Linux | 対応済 |
| iOS | 将来対応 |
| Android | 将来対応 |

## ビルド

```bash
# Rustコアのビルド
cargo build --release

# テスト実行
cargo test

# Tauri GUIビルド
cd src-tauri && cargo tauri build
```

## ライセンス

MIT License
