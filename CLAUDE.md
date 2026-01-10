# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## プロジェクト概要

P2P音声通信アプリ（macOS / Windows / Linux ネイティブ動作）

---

## 仕様書作成ガイドライン

本プロジェクトでは AI→AI パイプラインを前提とする。
- 仕様書を Claude Code が作成
- その仕様書を再び Claude Code が実装

### 基本原則

#### 1. AI→AI 向けの記述
- 感情的・抽象的な表現を避ける
- 判断理由を必ず明示する
- 「なぜそうするか」を残す（= ADR）

❌ 悪い例
```
高品質で低遅延な音声通話を実現する
```

⭕ 良い例
```
片方向エンドツーエンド遅延は 150ms 以下を目標とする。
codec は Opus 48kHz / 20ms frame を使用する。
```

#### 2. 仕様書 = 実装の正
- README や口頭説明より `docs/` を正とする
- 実装が仕様から逸脱する場合は必ず理由をコメントする

#### 3. 図解のルール
- mermaid で表現可能な図は mermaid を使用する
- mermaid に不向きな図（ASCII アート、複雑なレイアウト等）はその限りではない

mermaid 推奨ケース:
- フローチャート、シーケンス図、状態遷移図
- クラス図、ER 図
- アーキテクチャ概要図

mermaid 不向きケース:
- パケットフォーマットのバイナリレイアウト
- 細かい位置調整が必要な図
- 既存の ASCII アートで十分表現できている図

---

## 推奨ディレクトリ構成

```
/docs
 ├─ README.md              # 全体概要（軽量）
 ├─ architecture.md        # 最重要：技術構成
 ├─ adr/                   # 設計判断の固定
 │   ├─ ADR-001-*.md
 │   └─ ADR-002-*.md
 ├─ behavior/              # 振る舞い定義（BDD）
 │   ├─ *.feature
 └─ api/                   # 境界定義
     ├─ audio_engine.md
     ├─ signaling.md
```

---

## architecture.md 作成ルール

### 必須項目（決定事項のみ記載、選択肢や検討中の案は書かない）
- 対応 OS（macOS / Windows / Linux）
- 使用言語（例：C++20 / Rust stable）
- 音声 I/O
  - macOS: CoreAudio
  - Windows: WASAPI
  - Linux: ALSA / PipeWire
- ネットワーク方式（例：WebRTC Native）
- codec（Opus 等）
- スレッドモデル
- リアルタイム制約

### 曖昧さ排除ルール
- 「高速」「高品質」「低遅延」は禁止
- 数値 or 条件で書く

❌ `低遅延を目指す`
⭕ `RTT < 50ms 環境で片方向遅延 < 150ms`

---

## ADR（Architecture Decision Record）

### 役割
- Claude Code が勝手に設計を変えないための杭
- 後から見て「なぜそうなったか」を説明する

### テンプレート

```markdown
# ADR-XXX: <Decision Title>

## Context
なぜこの判断が必要だったか。

## Decision
何を採用 / 不採用にしたか。

## Consequences
この判断によるメリット・デメリット。
```

### 音声通信で必須になりやすいADR
- 通信方式（WebRTC / 独自実装）
- codec 選定
- ネイティブ UI 方針
- Electron / WebView を使わない判断
- リアルタイムスレッドの扱い

---

## BDD / Gherkin 作成ルール

### 目的
- 音声品質・ネットワーク劣化時の振る舞いを明文化
- テストやシミュレーションに直結させる

### 書き方
- 環境条件を `Given` に書く
- ユーザー操作 or イベントを `When`
- 観測可能な結果を `Then`

### 例

```gherkin
Scenario: Packet loss
  Given packet loss is 5%
  When audio streaming is active
  Then audio remains intelligible
```

※「intelligible」が何かは architecture or ADR 側で定義する

---

## API仕様作成ルール

### 目的
- UI / 音声エンジン / ネットワークの責務分離
- AI が境界を越えて実装しないようにする

### 記載項目
- API 名
- 入力
- 出力
- スレッド制約
- 呼び出しタイミング

### 例（audio_engine.md）

```
start_capture(device_id)
- Must be called from non-realtime thread
- Returns immediately
```

---

## 実装フェーズ指示

仕様書完成後、実装時は以下に従う：

- `docs/` 以下を正として実装する
- ADR に記載された判断は変更しない
- リアルタイム制約を破る場合は理由をコメントで残す
- 不明点は `TODO` として明示する

---

## Git コミットルール

### 言語
- コミットメッセージは英語で記述する

### 形式
- シンプル形式（feat:, fix: などのプレフィックスは使用しない）
- 1行目: 変更内容の要約（50文字以内を目安）
- 2行目: 空行
- 3行目以降: 必要に応じて詳細説明

### 書き方ルール
- 「何を」「なぜ」変更したかを明確に書く
- 1行目は命令形で書く（例: "Add", "Fix", "Change"）
- 曖昧な表現を避ける

❌ 悪い例
```
Fix
Bug fix
Various changes
```

⭕ 良い例
```
Change audio capture buffer size to 20ms

Adjusted frame size to keep latency under 150ms.
```

### コミット単位
- 1つの論理的な変更につき1コミット
- 複数の無関係な変更を1コミットに混ぜない
- ビルドが通る状態でコミットする

### 禁止事項
- 機密情報（APIキー、トークン等）をコミットしない
- 生成ファイル（build/, dist/ 等）をコミットしない（.gitignore で除外）
