---
sidebar_position: 1
title: jamjam について
description: ミュージシャン向け低遅延（< 2ms）P2P音声通信アプリ
---

:::note
このドキュメントは開発者向けの解説資料です。
正確な仕様・制約・判断は [docs-spec/](https://github.com/koedame/p2paudiosession/tree/main/docs-spec) を参照してください。
:::

# jamjam

ミュージシャン向け低遅延（< 2ms）P2P音声通信アプリ

## 概要

jamjamは、ミュージシャンがインターネット越しにリアルタイムでジャムセッションや遠隔レコーディングを行うためのアプリケーションです。

## 主な特徴

- **超低レイテンシ**: アプリケーション起因で2ms以下を目標（zero-latencyモード）
- **P2P通信**: 中央サーバーを介さない直接通信
- **クロスプラットフォーム**: Windows / macOS / Linux 対応
- **高音質**: 最大96kHz/32bit float対応、非圧縮PCMがデフォルト

## 対象ユースケース

- オンラインジャムセッション
- 遠隔レコーディング
- リアルタイム演奏セッション

## 対応プラットフォーム

| プラットフォーム | 状態 |
|-----------------|------|
| Windows | 対応済 |
| macOS | 対応済 |
| Linux | 対応済 |
| iOS | 将来対応 |
| Android | 将来対応 |

## 実装状況

| フェーズ | 機能 | 状態 |
|---------|------|------|
| Phase 1 | コア機能（オーディオエンジン、UDPトランスポート、プロトコル、CLI） | 完了 |
| Phase 2 | ネットワーク拡張（STUN、シグナリング、マルチピア、FEC） | 完了 |
| Phase 3 | Tauri GUI（デスクトップUI、ミキサー、設定画面） | 完了 |
| Phase 4 | 録音機能、メトロノーム共有 | 完了 |
| Phase 5 | エフェクト、VST/CLAPプラグインホスト | 完了 |

## 次のステップ

- [インストール](/docs/getting-started/installation) - jamjamをインストールする
- [クイックスタート](/docs/getting-started/quick-start) - 最初のセッションを始める
- [ジッタバッファ](/docs/concepts/jitter-buffer) - 低遅延を実現する技術を理解する
