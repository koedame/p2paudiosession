---
sidebar_label: Overview
sidebar_position: 1
---

<!-- このドキュメントは実装の正です。変更時は実装も同期すること -->

# jamjam Specification Documents

本ディレクトリは AI→AI パイプライン用の仕様書を格納する。
すべてのドキュメントは実装の唯一の正とする。

---

## ドキュメント構成

| ファイル | 説明 |
|---------|------|
| [architecture.md](./architecture.md) | 技術構成（最重要） |
| [ui-ux-guideline.md](./ui-ux-guideline.md) | UI/UXガイドライン |
| [adr/](./adr/ADR-001-language-rust.md) | 設計判断記録（ADR） |
| [api/](./api/audio_engine.md) | API境界定義 |
| behavior/ | 振る舞い定義（BDD/Gherkin） |

---

## ADR（設計判断記録）

| ADR | 決定内容 |
|-----|---------|
| [ADR-001](./adr/ADR-001-language-rust.md) | Rust採用 |
| [ADR-002](./adr/ADR-002-network-protocol.md) | カスタムUDPプロトコル採用 |
| [ADR-003](./adr/ADR-003-audio-codec.md) | 音声コーデック選択（複数対応） |
| [ADR-004](./adr/ADR-004-gui-framework.md) | GUIフレームワーク選択（Tauri / Flutter） |
| [ADR-005](./adr/ADR-005-no-audio-processing.md) | 音声処理を行わない方針 |
| [ADR-006](./adr/ADR-006-fec-strategy.md) | FEC（前方誤り訂正）採用 |
| [ADR-007](./adr/ADR-007-i18n-library.md) | i18nライブラリ選定 |
| [ADR-008](./adr/ADR-008-zero-latency-mode.md) | ゼロレイテンシーモード |

---

## API仕様

| API | 説明 |
|-----|------|
| [audio_engine.md](./api/audio_engine.md) | オーディオエンジンAPI |
| [network.md](./api/network.md) | ネットワークAPI |
| [signaling.md](./api/signaling.md) | シグナリングAPI |
| [plugin.md](./api/plugin.md) | プラグインホストAPI |
| [i18n.md](./api/i18n.md) | 国際化API |

---

## BDD仕様

| Feature | 説明 |
|---------|------|
| [connection.feature](./behavior/connection.feature) | セッション接続 |
| [audio-quality.feature](./behavior/audio-quality.feature) | 音声品質 |
| [latency.feature](./behavior/latency.feature) | 遅延管理 |
| [i18n.feature](./behavior/i18n.feature) | 国際化 |

---

## 更新ルール

1. **実装が仕様に影響を与える変更を行った場合、同一コミットで仕様書も更新する**
2. ADR は追加のみ（既存 ADR の変更は原則禁止、変更が必要な場合は新規 ADR を作成）
3. API 仕様は実装と常に同期を維持する
4. BDD 仕様はテストコードと対応させる

---

## 開発者向けドキュメント

開発者向けの解説資料（ガイド、チュートリアル等）は [Docs](/docs/intro) を参照。

> docs-site/ の内容は仕様ではない。実装の正は常に本ディレクトリ（docs-spec/）である。
