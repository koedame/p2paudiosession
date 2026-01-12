---
sidebar_position: 1
title: インストール
description: jamjamのインストール方法
---

:::note
このドキュメントは開発者向けの解説資料です。
正確な仕様・制約・判断は [docs-spec/](https://github.com/koedame/p2paudiosession/tree/main/docs-spec) を参照してください。
:::

# インストール

jamjamをインストールする方法を説明します。

## システム要件

### 対応OS

- Windows 10/11 (64-bit)
- macOS 11.0 (Big Sur) 以降
- Linux (Ubuntu 22.04, Fedora 38 等)

### ハードウェア要件

- オーディオインターフェース（ASIO/CoreAudio/ALSA対応）
- 安定したインターネット接続

## インストール方法

### リリースビルドからのインストール

1. [GitHub Releases](https://github.com/koedame/p2paudiosession/releases) から最新版をダウンロード
2. 各プラットフォーム用のインストーラを実行:
   - Windows: `.msi` または `.exe`
   - macOS: `.dmg`
   - Linux: `.AppImage` または `.deb`

### ソースからのビルド

開発版を使用する場合は、ソースからビルドします。

```bash
# リポジトリをクローン
git clone https://github.com/koedame/p2paudiosession.git
cd p2paudiosession

# Rustコアのビルド
cargo build --release

# Tauri GUIビルド（プロジェクトルートから実行）
cargo tauri build
```

#### ビルド依存関係

**Ubuntu/Debian:**
```bash
sudo apt-get update
sudo apt-get install -y libasound2-dev libssl-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

**macOS:**
- Xcode Command Line Tools

**Windows:**
- Visual Studio Build Tools (MSVC)

## 次のステップ

インストールが完了したら、[クイックスタート](/docs/getting-started/quick-start)に進んでください。
