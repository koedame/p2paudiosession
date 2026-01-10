---
sidebar_position: 2
title: テスト
description: jamjamのテスト実行方法
---

:::note
このドキュメントは開発者向けの解説資料です。
正確な仕様・制約・判断は [docs-spec/](https://github.com/koedame/p2paudiosession/tree/main/docs-spec) を参照してください。
:::

# テスト

jamjamのテストを実行する方法を説明します。

## テストの実行

### 全テスト実行

```bash
cargo test --all-targets
```

### 特定のテストを実行

```bash
# テスト名で絞り込み
cargo test test_name

# 特定のモジュールのテスト
cargo test audio::
cargo test network::
```

### テストの詳細出力

```bash
cargo test -- --nocapture
```

## テスト構成

テストは [docs-spec/behavior/](https://github.com/koedame/p2paudiosession/tree/main/docs-spec/behavior) の BDD 仕様に基づいています。

| テストファイル | 対応仕様 |
|--------------|---------|
| `tests/connection_test.rs` | `docs-spec/behavior/connection.feature` |
| `tests/audio_quality_test.rs` | `docs-spec/behavior/audio-quality.feature` |
| `tests/latency_test.rs` | `docs-spec/behavior/latency.feature` |
| `tests/i18n_test.rs` | `docs-spec/behavior/i18n.feature` |

## コード品質チェック

### フォーマットチェック

```bash
cargo fmt --check
```

### Lint (Clippy)

```bash
cargo clippy --all-targets -- -D warnings
```

### 全チェック（CI相当）

```bash
cargo fmt --check && \
cargo clippy --all-targets -- -D warnings && \
cargo check --all-targets && \
cargo test --all-targets
```

## 関連情報

- [ビルド](/docs/development/building) - ビルド方法
- [CI/CD](/docs/development/ci) - 継続的インテグレーション
