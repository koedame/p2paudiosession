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

## ✅ フェーズ1-3: 完了 `cc:DONE`

設定永続化、招待コード、プリセット選択、エラーメッセージ改善、入力レベルメーター、ミュート機能、接続履歴、ダークモード対応が完了。

---

## ✅ フェーズ4: 仕上げ `cc:DONE`

### E2Eテスト ✅
- `tests/signaling_e2e_test.rs` - 10件のE2Eテスト
- 接続→音声送受信→切断フロー、エラーケース（タイムアウト、接続拒否等）

### テストユーザー向けマニュアル ✅
- `docs-site/docs/getting-started/installation.md` - インストール方法
- `docs-site/docs/getting-started/quick-start.md` - 基本的な使い方
- `docs-site/docs/getting-started/troubleshooting.md` - トラブルシューティング

### デスクトップビルド確認
- [x] Linux (Ubuntu) - `.deb`, `.rpm`, `.AppImage` 生成確認
- [x] macOS - GitHub Actions CI（`.github/workflows/build.yml`）
- [x] Windows - GitHub Actions CI（`.github/workflows/build.yml`）

### コードレビュー ✅
- セキュリティ確認、パフォーマンス確認、コード品質確認完了

---

## 残タスク（オプション）

- [ ] カスタムプリセット保存機能
- [ ] キーボードショートカット（スペースキーでミュート等）

---

## 完了したタスク

### 2026-01-15

- [x] **フェーズ4完了: 仕上げ**
  - E2Eテスト: `tests/signaling_e2e_test.rs` 10件追加
  - マニュアル: troubleshooting.md新規作成、quick-start.md更新
  - Linuxビルド確認、コードレビュー完了
  - macOS/Windows: GitHub Actions CI設定（PRトリガー追加）

- [x] **フェーズ1-3完了**
  - 設定永続化、招待コード、プリセット、エラーメッセージ改善
  - 入力レベルメーター、ミュート、接続履歴、ダークモード

---

## 参照

- [AGENTS.md](./AGENTS.md) - 開発ワークフロー
- [docs-spec/](./docs-spec/) - 仕様書
