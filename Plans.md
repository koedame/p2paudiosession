# Plans.md - タスク管理

> このファイルは Claude Code がタスクを管理するために使用します。
> `/plan-with-agent` でタスクを追加、`/work` で実行します。

## 凡例

- `[ ]` 未着手
- `[>]` 進行中
- `[x]` 完了
- `[!]` ブロック中
- `[-]` キャンセル

---

## 🎯 プロジェクト: テストユーザー検証準備

### 概要
- **目的**: X（Twitter）でテストユーザーを募集し、音楽セッション＋通話の両方で検証できる状態にする
- **対象**: 適正のある数人のテストユーザー（技術サポートあり）
- **スコープ**: 製品レベル品質

### 技術スタック（既存）
- フロントエンド: React + TypeScript (Tauri 2.0)
- バックエンド: Rust
- ネットワーク: カスタムUDPプロトコル
- シグナリング: WebSocket over TLS

---

## ✅ フェーズ1: 基盤改善（必須） `cc:DONE`

### 設定永続化 `[feature:tdd]`

- [x] [impl] 設定ファイル読み書き機能の実装（TOML形式）
  - 保存先: `~/.config/jamjam/config.toml`（Linux）/ `%APPDATA%\jamjam\config.toml`（Windows）/ `~/Library/Application Support/jamjam/config.toml`（macOS）
  - 参照: docs-spec/architecture.md セクション13
- [x] [impl] 入力/出力デバイスID、バッファサイズの永続化
- [x] [impl] デフォルトシグナリングサーバーURLの永続化
- [x] [impl] アプリ起動時に設定を自動読み込み
- [x] [test] 設定の読み書きテスト

### 招待コード（6文字英数字） `[feature:tdd]`

- [x] [impl] シグナリングサーバー側で招待コード生成
  - UUIDとは別に6文字英数字の招待コードを生成
  - 参照: docs-spec/architecture.md セクション7.1
- [x] [impl] 招待コードでルーム参加できるUIの追加
  - 現在のルームリストに加えて「コードで参加」入力欄
- [x] [test] 招待コード生成・参加のテスト

---

## 🟡 フェーズ2: コア体験（必須） `cc:TODO`

### プリセット選択UI

- [ ] [impl] プリセット一覧表示コンポーネント
  - zero-latency, ultra-low-latency, balanced, high-quality
  - 参照: docs-spec/architecture.md セクション8.3
- [ ] [impl] プリセット選択時の設定自動適用
  - Jitterバッファ、コーデック、フレームサイズの自動設定
- [ ] [impl] 現在選択中のプリセットの保存・読み込み
- [ ] [impl] カスタムプリセット保存機能

### 接続品質→推奨設定表示 `[feature:tdd]`

- [ ] [impl] ジッター値に基づく推奨プリセット判定ロジック
  - < 1ms: zero-latency推奨
  - 1-3ms: ultra-low-latency推奨
  - 3-10ms: balanced推奨
  - > 10ms: high-quality推奨
  - 参照: docs-spec/architecture.md セクション8.1「接続品質モニタリング」
- [ ] [impl] 推奨設定の表示UI（メイン画面に追加）
- [ ] [impl] 推奨に切り替えるボタン
- [ ] [test] 推奨判定ロジックのテスト

### エラーメッセージ改善

- [ ] [impl] 技術エラーをユーザーフレンドリーなメッセージに変換
  - Connection refused → 「サーバーに接続できません。URLを確認してください」
  - Timeout → 「接続がタイムアウトしました。ネットワークを確認してください」
- [ ] [impl] i18n対応（日本語/英語）
- [ ] [test] エラーメッセージ変換のテスト

---

## 🟢 フェーズ3: UX改善（推奨） `cc:TODO`

### 入力レベルメーター

- [ ] [impl] 音声入力レベルのリアルタイム取得API
- [ ] [impl] VUメーターコンポーネント（設定画面）
- [ ] [impl] 接続中のメイン画面にもミニメーター表示

### ミュート機能

- [ ] [impl] マイクミュート/アンミュート機能
  - ローカルで音声送信を停止（サーバー側処理不要）
- [ ] [impl] ミュートボタンUI（メイン画面）
- [ ] [impl] キーボードショートカット（スペースキー等）

### 接続履歴

- [ ] [impl] 過去に接続したサーバーURLの履歴保存
- [ ] [impl] サーバーURL入力欄のオートコンプリート
- [ ] [impl] 「最近の接続」リスト表示

### ダークモード対応

- [ ] [impl] カラーテーマの抽象化（CSS変数）
- [ ] [impl] ダークテーマの定義
- [ ] [impl] テーマ切り替えUI（設定画面）
- [ ] [impl] システム設定に追従するオプション
- [ ] [impl] 選択テーマの永続化

### アプリアイコン/スプラッシュ

- [ ] [design] アプリアイコンのデザイン
- [ ] [impl] macOS/Windows/Linux用アイコン設定
- [ ] [impl] スプラッシュスクリーン（必要に応じて）

---

## 🔵 フェーズ4: 仕上げ `cc:TODO`

- [ ] [test] E2Eテスト（接続→音声送受信→切断）
- [ ] [docs] テストユーザー向け簡易マニュアル作成
- [ ] [infra] デスクトップビルド（macOS/Windows/Linux）の動作確認
- [ ] [review] `/harness-review` でコードレビュー

---

## 完了したタスク

<!-- 完了したタスクはここに移動 -->

### 2026-01-15

- [x] **フェーズ1完了: 基盤改善**
  - 設定永続化（TOML形式、プラットフォーム別パス対応）
    - `src-tauri/src/config.rs` - 設定管理モジュール
    - `ui/src/lib/tauri.ts` - Config API追加
  - 招待コード機能（6文字英数字、0/O/I/1/L除外）
    - `src/network/signaling.rs` - 招待コード生成・検証
    - `src/bin/signaling_server.rs` - サーバー側対応
    - `src-tauri/src/signaling.rs` - Tauri IPC対応
  - フロントエンドUI
    - MainScreen: 招待コード入力・表示UI
    - SettingsScreen: 設定変更時の永続化
  - テスト: 141テスト全てパス

### 2025-01-15

- [x] test-server-deploy スキルにCLI接続確認を追加
- [x] テストサーバーへデプロイ

---

## 参照

- [AGENTS.md](./AGENTS.md) - 開発ワークフロー
- [docs-spec/](./docs-spec/) - 仕様書
- [docs-spec/architecture.md](./docs-spec/architecture.md) - 技術構成
