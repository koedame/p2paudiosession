# AGENTS.md - 開発ワークフロー

このプロジェクトの開発フローと責務を定義する。

## ドキュメント体系

```
CLAUDE.md          # Claude Code 設定・プロジェクト概要
AGENTS.md          # 開発ワークフロー（本ファイル）
Plans.md           # タスク管理

docs-spec/         # 仕様書（実装の唯一の正）
├── architecture.md    # 技術構成（最重要）
├── adr/              # 設計判断記録
├── api/              # API境界定義
└── behavior/         # BDD仕様

docs-site/         # 開発者向けドキュメント（解説資料）

.claude/
├── settings.json     # 権限設定
├── skills/           # スキル定義
├── memory/           # セッション間記憶
│   ├── decisions.md  # 意思決定メモ
│   └── patterns.md   # 再利用パターン
└── rules/            # 品質保護ルール
```

## 開発フロー（Solo Mode）

```
┌─────────────────────────────────────────────────────────┐
│                    開発サイクル                          │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  1. Plan（計画）                                        │
│     └─ /plan-with-agent で Plans.md にタスク追加        │
│                                                         │
│  2. Work（実装）                                        │
│     └─ /work で Plans.md のタスクを実行                 │
│                                                         │
│  3. Review（レビュー）                                  │
│     └─ /review でコード品質チェック                     │
│                                                         │
│  4. Verify（検証）                                      │
│     └─ cargo test && cargo clippy でビルド検証          │
│                                                         │
│  5. Commit（コミット）                                  │
│     └─ /commit でコミット作成                           │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## 主要スキル

| スキル | 用途 |
|--------|------|
| `/plan-with-agent` | タスク計画の作成 |
| `/work` | Plans.md のタスクを実行 |
| `/review` | コードレビュー |
| `/commit` | コミットメッセージ生成 |
| `/sync-spec` | 仕様と実装の同期チェック |
| `/test-server-deploy` | テストサーバーへデプロイ |

## 仕様書ルール

1. **docs-spec/ が実装の唯一の正**
   - 実装が仕様と矛盾する場合、仕様に合わせて実装を修正
   - 仕様変更が必要な場合は新規ADRを作成

2. **ADRは追加のみ**
   - 既存ADRの変更は原則禁止
   - 決定を変更する場合は新規ADRで上書き

3. **同期ルール**
   - 実装変更時は同一コミットで仕様書も更新
   - `/sync-spec` で定期的に整合性チェック

## 品質基準

- `cargo fmt --check` - フォーマットチェック
- `cargo clippy -- -D warnings` - 静的解析
- `cargo test` - テスト実行

すべてパスしないとコミット不可。

## メモリシステム

### decisions.md（意思決定メモ）
- セッション中の小さな決定を記録
- 重要な決定はADRに昇格

### patterns.md（再利用パターン）
- コードパターン、ベストプラクティス
- よく使うコマンド・手順

## 参照

- [architecture.md](./docs-spec/architecture.md) - 技術構成
- [ADR一覧](./docs-spec/README.md#adr設計判断記録) - 設計判断
- [API仕様](./docs-spec/README.md#api仕様) - API境界
