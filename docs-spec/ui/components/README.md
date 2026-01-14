# コンポーネントカタログ

jamjam で使用する UI コンポーネントの一覧と仕様。

---

## コンポーネント一覧

| コンポーネント | 優先度 | ステータス | 仕様 |
|--------------|--------|----------|------|
| ConnectionIndicator | P0 | 仕様作成済 | [connection-indicator.md](./connection-indicator.md) |
| Button | P0 | 仕様未作成 | - |
| Input | P0 | 仕様未作成 | - |
| MixerChannel | P0 | 仕様未作成 | - |
| PresetSelector | P1 | 仕様未作成 | - |
| Toast | P1 | 仕様未作成 | - |
| Modal | P1 | 仕様未作成 | - |
| LevelMeter | P1 | 仕様未作成 | - |
| VolumeSlider | P1 | 仕様未作成 | - |

---

## コンポーネント仕様テンプレート

各コンポーネント仕様には以下を含める：

### 1. 概要
- 目的
- 使用場面

### 2. Props / API

```typescript
interface ComponentProps {
  required: string;        // 必須プロパティ
  optional?: string;       // オプション
  onEvent?: () => void;    // イベントハンドラ
}
```

### 3. 状態とバリアント

| バリアント | 用途 |
|-----------|------|
| primary | 主要アクション |
| secondary | 補助アクション |

### 4. ビジュアル仕様

```
┌─────────────────┐
│  [ボタン]       │
└─────────────────┘
```

### 5. アクセシビリティ

- ARIA属性
- キーボード操作
- スクリーンリーダー対応

### 6. i18n キー

使用する翻訳キーの一覧

### 7. 使用例

```tsx
<Component variant="primary" onClick={handleClick}>
  ラベル
</Component>
```

---

## 基本コンポーネント

### Button

**用途**: クリック可能なアクション

**バリアント**:
| バリアント | 用途 | 背景色 |
|-----------|------|--------|
| primary | 主要アクション（ルーム作成等） | `--color-accent` |
| secondary | 補助アクション（キャンセル等） | `--color-bg-elevated` |
| danger | 破壊的アクション（退出等） | `--color-danger` |
| ghost | テキストのみ | 透明 |

**サイズ**:
| サイズ | パディング | フォントサイズ |
|-------|-----------|---------------|
| sm | `--space-xs` `--space-sm` | `--font-size-caption` |
| md | `--space-sm` `--space-md` | `--font-size-body` |
| lg | `--space-md` `--space-lg` | `--font-size-h3` |

**状態**:
- default
- hover
- active
- disabled
- loading

---

### Input

**用途**: テキスト入力

**タイプ**:
- text: 通常のテキスト
- code: 招待コード入力（6文字、大文字変換）
- password: パスワード入力

**状態**:
- default
- focus
- error
- disabled

---

### Toast

**用途**: 一時的な通知メッセージ

**タイプ**:
| タイプ | 用途 | アイコン | 背景色 |
|-------|------|---------|--------|
| success | 成功通知 | ✓ | `--color-success-bg` |
| warning | 警告 | ⚠ | `--color-warning-bg` |
| error | エラー | ✕ | `--color-danger-bg` |
| info | 情報 | ℹ | `--color-bg-elevated` |

**動作**:
- 右上に表示
- 3秒後に自動消去（エラーは手動消去のみ）
- 複数表示時はスタック

---

### Modal

**用途**: オーバーレイダイアログ

**注意**: 演奏中（セッションアクティブ時）はモーダル表示禁止

**構造**:
```
┌────────────────────────────┐
│ [✕]               タイトル │
├────────────────────────────┤
│                            │
│     [ コンテンツ ]         │
│                            │
├────────────────────────────┤
│      [キャンセル] [OK]     │
└────────────────────────────┘
```

**アクセシビリティ**:
- `role="dialog"`
- `aria-modal="true"`
- フォーカストラップ
- Escapeで閉じる

---

## ドメイン固有コンポーネント

### ConnectionIndicator

接続状態を色とアイコンで表示。

→ 詳細: [connection-indicator.md](./connection-indicator.md)

---

### MixerChannel

参加者ごとの音量コントロール。

**構成要素**:
- 参加者名
- レベルメーター（縦型）
- 音量フェーダー
- ミュートボタン
- ソロボタン（オプション）

**サイズ**:
- 幅: 80px
- 高さ: 親要素に合わせる

---

### PresetSelector

プリセット選択 UI。

**表示形式**: ラジオボタンリスト

**各項目**:
- ラジオボタン
- プリセット名（日本語）
- 説明文（1行）
- 推奨バッジ（該当する場合）

---

### LevelMeter

音声レベルをリアルタイム表示。

**向き**: 縦型（ミキサー用）、横型（インライン用）

**色分け**:
| レベル | 色 |
|-------|-----|
| -∞ to -12dB | `--color-meter-low` (緑) |
| -12 to -3dB | `--color-meter-mid` (黄) |
| -3 to 0dB | `--color-meter-high` (赤) |

**更新頻度**: 60fps（演奏中は30fpsに制限）

---

### VolumeSlider

音量調整用スライダー。

**向き**: 縦型（ミキサー）、横型（マスター）

**範囲**: 0% - 100%

**デフォルト**: 80%

**アクセシビリティ**:
- `role="slider"`
- `aria-valuemin="0"`
- `aria-valuemax="100"`
- `aria-valuenow="80"`
- `aria-valuetext="80パーセント"`

---

## 実装ガイドライン

### ファイル構成

```
ui/src/components/
├── Button/
│   ├── Button.tsx
│   ├── Button.css
│   └── index.ts
├── ConnectionIndicator/
│   ├── ConnectionIndicator.tsx
│   ├── ConnectionIndicator.css
│   └── index.ts
└── index.ts  # 全コンポーネントの再エクスポート
```

### 命名規則

- コンポーネント: PascalCase（`ConnectionIndicator`）
- ファイル: コンポーネント名と同じ
- CSS クラス: BEM または コンポーネントスコープ

### スタイリング

- CSS Custom Properties（design-tokens.md 参照）を使用
- コンポーネント固有のCSSは同ディレクトリに配置
- グローバルスタイルは `styles/` に配置
