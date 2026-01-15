# patterns.md - 再利用パターン

> よく使うコードパターン、コマンド、ベストプラクティスを記録する。

---

## ビルド・テスト

```bash
# フォーマットチェック
cargo fmt --check

# 静的解析
cargo clippy -- -D warnings

# テスト実行
cargo test

# リリースビルド
cargo build --release
```

## デプロイ

```bash
# テストサーバーへデプロイ
/test-server-deploy
```

## Git ワークフロー

```bash
# コミット作成
/commit

# プッシュ（複数 upstream があるため明示的に指定）
git push origin main
```

## 仕様同期

```bash
# 仕様と実装の整合性チェック
/sync-spec
```

---

## コードパターン

### エラーハンドリング（Rust）

```rust
// Result を返す関数
pub fn example() -> Result<(), Error> {
    // エラー伝播には ? を使用
    some_operation()?;
    Ok(())
}

// thiserror でエラー型定義
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("operation failed: {0}")]
    OperationFailed(String),
}
```

### 非同期処理（tokio）

```rust
#[tokio::main]
async fn main() {
    // 並列実行
    let (a, b) = tokio::join!(task_a(), task_b());
}
```

---

<!-- 新しいパターンはここに追加 -->
