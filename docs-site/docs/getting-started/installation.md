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

:::caution 署名なしアプリの警告について
現在配布しているビルドはコード署名されていないため、OSのセキュリティ機能により警告が表示されます。
以下の手順で起動できます。
:::

#### Windows での起動方法

Windows Defender SmartScreen の警告が表示された場合:

1. 「詳細情報」をクリック
2. 「実行」ボタンをクリック

#### macOS での起動方法

「開発元を検証できないため開けません」と表示された場合:

1. **Finder** でアプリケーションを右クリック（または Control + クリック）
2. 「開く」を選択
3. 確認ダイアログで「開く」をクリック

または、システム設定から許可する方法:

1. **システム設定** → **プライバシーとセキュリティ**
2. 「jamjamは開発元を確認できないため、使用がブロックされました」の横にある「このまま開く」をクリック

#### Linux での起動方法

AppImage の場合、実行権限を付与してから起動:

```bash
chmod +x jamjam_*.AppImage
./jamjam_*.AppImage
```

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
