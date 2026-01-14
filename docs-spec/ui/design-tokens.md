# デザイントークン

jamjam UI で使用する CSS Custom Properties（CSS変数）の定義。

> **実装先**: `ui/src/styles/tokens.css`

---

## 概要

すべてのスタイリングは CSS 変数を通じて行う。
これにより、テーマ切り替え（ダーク/ライト）が容易になり、
デザインの一貫性が保たれる。

### 命名規則

```
--{category}-{property}-{variant}

例:
--color-bg-primary
--space-md
--radius-lg
```

---

## カラートークン

### ダークテーマ（デフォルト）

```css
:root,
:root[data-theme="dark"] {
  /* === Background === */
  --color-bg-primary: #1E1E2E;      /* メイン背景 */
  --color-bg-secondary: #2A2A3E;    /* カード背景 */
  --color-bg-elevated: #363652;     /* ホバー・フォーカス */
  --color-bg-overlay: rgba(0, 0, 0, 0.5);  /* モーダル背景 */

  /* === Accent === */
  --color-accent: #7C5CFF;          /* プライマリアクション */
  --color-accent-hover: #9B7FFF;    /* ホバー時 */
  --color-accent-active: #6B4FE0;   /* アクティブ時 */

  /* === Semantic === */
  --color-success: #4ADE80;         /* 接続成功、正常状態 */
  --color-success-bg: rgba(74, 222, 128, 0.1);
  --color-warning: #FBBF24;         /* 警告、不安定 */
  --color-warning-bg: rgba(251, 191, 36, 0.1);
  --color-danger: #F87171;          /* エラー、切断 */
  --color-danger-bg: rgba(248, 113, 113, 0.1);

  /* === Text === */
  --color-text-primary: #FFFFFF;    /* 主要テキスト */
  --color-text-secondary: #A0A0B0;  /* 補助テキスト */
  --color-text-disabled: #606070;   /* 無効状態 */
  --color-text-inverse: #1E1E2E;    /* 反転テキスト */

  /* === Border === */
  --color-border: #404060;          /* 通常ボーダー */
  --color-border-focus: #7C5CFF;    /* フォーカス時 */
  --color-border-hover: #505070;    /* ホバー時 */

  /* === Connection Status === */
  --color-status-disconnected: #606070;   /* グレー */
  --color-status-connecting: #FBBF24;     /* 黄（点滅） */
  --color-status-connected: #4ADE80;      /* 緑 */
  --color-status-unstable: #FBBF24;       /* 黄 */
  --color-status-error: #F87171;          /* 赤 */

  /* === Mixer === */
  --color-meter-low: #4ADE80;       /* -∞ to -12dB */
  --color-meter-mid: #FBBF24;       /* -12 to -3dB */
  --color-meter-high: #F87171;      /* -3dB to 0dB */
  --color-mute-active: #F87171;     /* ミュートON */
  --color-solo-active: #FBBF24;     /* ソロON */
}
```

### ライトテーマ

```css
:root[data-theme="light"] {
  /* === Background === */
  --color-bg-primary: #FAFAFA;
  --color-bg-secondary: #FFFFFF;
  --color-bg-elevated: #F0F0F5;
  --color-bg-overlay: rgba(0, 0, 0, 0.3);

  /* === Accent === */
  --color-accent: #6B4FE0;
  --color-accent-hover: #5A3FD0;
  --color-accent-active: #4A2FC0;

  /* === Semantic === */
  --color-success: #22C55E;
  --color-success-bg: rgba(34, 197, 94, 0.1);
  --color-warning: #EAB308;
  --color-warning-bg: rgba(234, 179, 8, 0.1);
  --color-danger: #EF4444;
  --color-danger-bg: rgba(239, 68, 68, 0.1);

  /* === Text === */
  --color-text-primary: #1E1E2E;
  --color-text-secondary: #606070;
  --color-text-disabled: #A0A0B0;
  --color-text-inverse: #FFFFFF;

  /* === Border === */
  --color-border: #E0E0E8;
  --color-border-focus: #6B4FE0;
  --color-border-hover: #D0D0D8;

  /* === Connection Status === */
  --color-status-disconnected: #A0A0B0;
  --color-status-connecting: #EAB308;
  --color-status-connected: #22C55E;
  --color-status-unstable: #EAB308;
  --color-status-error: #EF4444;

  /* === Mixer === */
  --color-meter-low: #22C55E;
  --color-meter-mid: #EAB308;
  --color-meter-high: #EF4444;
  --color-mute-active: #EF4444;
  --color-solo-active: #EAB308;
}
```

---

## タイポグラフィトークン

```css
:root {
  /* === Font Family === */
  --font-family-sans:
    -apple-system,
    BlinkMacSystemFont,
    'Segoe UI',
    Roboto,
    'Hiragino Sans',
    'Hiragino Kaku Gothic ProN',
    'Noto Sans JP',
    sans-serif;

  --font-family-mono:
    'SF Mono',
    'Fira Code',
    Consolas,
    'Courier New',
    monospace;

  /* === Font Size === */
  --font-size-h1: 24px;
  --font-size-h2: 18px;
  --font-size-h3: 16px;
  --font-size-body: 14px;
  --font-size-caption: 12px;
  --font-size-small: 11px;

  /* === Font Weight === */
  --font-weight-bold: 700;
  --font-weight-semibold: 600;
  --font-weight-normal: 400;

  /* === Line Height === */
  --line-height-tight: 1.25;
  --line-height-normal: 1.5;
  --line-height-relaxed: 1.75;

  /* === Letter Spacing === */
  --letter-spacing-tight: -0.02em;
  --letter-spacing-normal: 0;
  --letter-spacing-wide: 0.02em;
}
```

### タイポグラフィの使用例

```css
/* 見出し1 */
.h1 {
  font-family: var(--font-family-sans);
  font-size: var(--font-size-h1);
  font-weight: var(--font-weight-bold);
  line-height: var(--line-height-tight);
}

/* 本文 */
.body {
  font-family: var(--font-family-sans);
  font-size: var(--font-size-body);
  font-weight: var(--font-weight-normal);
  line-height: var(--line-height-normal);
}

/* 数値表示（遅延時間など） */
.numeric {
  font-family: var(--font-family-mono);
  font-size: var(--font-size-body);
  font-weight: var(--font-weight-semibold);
}
```

---

## スペーシングトークン

4px ベースのスペーシングシステム。

```css
:root {
  /* === Base Unit === */
  --space-unit: 4px;

  /* === Spacing Scale === */
  --space-xs: 4px;      /* 1 unit */
  --space-sm: 8px;      /* 2 units */
  --space-md: 16px;     /* 4 units */
  --space-lg: 24px;     /* 6 units */
  --space-xl: 32px;     /* 8 units */
  --space-2xl: 48px;    /* 12 units */
  --space-3xl: 64px;    /* 16 units */

  /* === Component Padding === */
  --padding-button: var(--space-sm) var(--space-md);
  --padding-card: var(--space-md);
  --padding-input: var(--space-sm) var(--space-sm);
  --padding-modal: var(--space-lg);

  /* === Layout === */
  --gap-xs: var(--space-xs);
  --gap-sm: var(--space-sm);
  --gap-md: var(--space-md);
  --gap-lg: var(--space-lg);
}
```

---

## 角丸トークン

```css
:root {
  --radius-none: 0;
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --radius-xl: 16px;
  --radius-full: 9999px;  /* 円形ボタン用 */
}
```

### 使用ガイドライン

| コンポーネント | 角丸 |
|--------------|------|
| ボタン | `--radius-md` |
| カード | `--radius-lg` |
| 入力フィールド | `--radius-sm` |
| モーダル | `--radius-xl` |
| バッジ・タグ | `--radius-full` |
| アイコンボタン（円形） | `--radius-full` |

---

## シャドウトークン

```css
:root {
  --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.1);
  --shadow-md: 0 4px 6px rgba(0, 0, 0, 0.15);
  --shadow-lg: 0 10px 15px rgba(0, 0, 0, 0.2);
  --shadow-xl: 0 20px 25px rgba(0, 0, 0, 0.25);

  /* Focus ring */
  --shadow-focus: 0 0 0 3px rgba(124, 92, 255, 0.4);
}
```

---

## アニメーショントークン

```css
:root {
  /* === Duration === */
  --duration-instant: 0ms;
  --duration-fast: 100ms;
  --duration-normal: 200ms;
  --duration-slow: 300ms;
  --duration-slower: 500ms;

  /* === Easing === */
  --ease-default: ease-out;
  --ease-in: ease-in;
  --ease-in-out: ease-in-out;
  --ease-bounce: cubic-bezier(0.68, -0.55, 0.265, 1.55);

  /* === Transition Presets === */
  --transition-fast: var(--duration-fast) var(--ease-default);
  --transition-normal: var(--duration-normal) var(--ease-default);
  --transition-slow: var(--duration-slow) var(--ease-default);
}

/* アクセシビリティ: motion 設定を尊重 */
@media (prefers-reduced-motion: reduce) {
  :root {
    --duration-instant: 0ms;
    --duration-fast: 0ms;
    --duration-normal: 0ms;
    --duration-slow: 0ms;
    --duration-slower: 0ms;
  }
}
```

### アニメーション使用ガイドライン

| シーン | Duration | 理由 |
|-------|----------|------|
| ボタンホバー | fast | 即時フィードバック |
| モーダル表示 | normal | スムーズな遷移 |
| ページ遷移 | slow | 視覚的区切り |
| 接続中スピナー | slower | 穏やかな回転 |

**注意**: 演奏中（セッションアクティブ時）はアニメーションを最小限に抑える。

---

## Z-Index トークン

```css
:root {
  --z-base: 0;
  --z-dropdown: 100;
  --z-sticky: 200;
  --z-overlay: 300;
  --z-modal: 400;
  --z-popover: 500;
  --z-tooltip: 600;
  --z-toast: 700;
}
```

---

## レスポンシブトークン

```css
:root {
  /* === Breakpoints === */
  --breakpoint-sm: 640px;   /* Mobile */
  --breakpoint-md: 768px;   /* Tablet */
  --breakpoint-lg: 1024px;  /* Desktop */
  --breakpoint-xl: 1280px;  /* Large Desktop */

  /* === Container === */
  --container-sm: 640px;
  --container-md: 768px;
  --container-lg: 1024px;
  --container-xl: 1200px;
}
```

### レスポンシブ使用例

```css
/* Mobile first */
.container {
  padding: var(--space-sm);
}

@media (min-width: 768px) {
  .container {
    padding: var(--space-md);
  }
}
```

---

## 実装チェックリスト

### ファイル作成

```bash
ui/src/styles/
├── tokens.css       # このドキュメントの CSS
├── reset.css        # CSS リセット
└── global.css       # グローバルスタイル（tokens.css をインポート）
```

### 使用規則

- [ ] 生のカラー値（`#7C5CFF`）を直接使用しない
- [ ] 生のピクセル値（`16px`）を直接使用しない（例外: ボーダー幅）
- [ ] テーマ切り替えは `data-theme` 属性で行う
- [ ] `prefers-reduced-motion` を尊重する

### テーマ切り替え実装

```typescript
// テーマ切り替え
function setTheme(theme: 'dark' | 'light') {
  document.documentElement.setAttribute('data-theme', theme);
  localStorage.setItem('theme', theme);
}

// 初期化（システム設定を尊重）
function initTheme() {
  const saved = localStorage.getItem('theme');
  if (saved) {
    setTheme(saved as 'dark' | 'light');
  } else if (window.matchMedia('(prefers-color-scheme: light)').matches) {
    setTheme('light');
  } else {
    setTheme('dark');
  }
}
```
