# This specification is the source of truth. Sync implementation when changed.

Feature: 遅延管理
  jamjamでの遅延に関する振る舞い

  Background:
    Given jamjamアプリケーションが起動している
    And オーディオデバイスが正常に認識されている
    And セッションに接続済み

  # 遅延表示
  Scenario: 遅延情報を表示する
    Given 接続が確立している
    Then 以下の遅延情報が表示される:
      | 項目 | 説明 |
      | ネットワークRTT | 相手との往復遅延（ms） |
      | Jitterバッファ | 現在のバッファサイズ（ms） |
      | 総遅延 | 片道の推定総遅延（ms） |

  Scenario: 複数参加者の遅延を個別表示する
    Given 3名でセッション中
    Then 各参加者との遅延が個別に表示される
    And 最も遅延が大きい参加者がハイライトされる

  # Jitterバッファ
  Scenario: Jitterバッファが適応的に調整される
    Given Jitterバッファが「適応的」に設定されている
    And 初期バッファサイズが4フレーム
    When ネットワークジッターが増加する
    Then Jitterバッファサイズが自動的に増加する
    And 「バッファサイズを調整しました」と通知される

  Scenario: Jitterバッファが最小サイズを下回らない
    Given Jitterバッファの最小サイズが2フレームに設定されている
    When ネットワークが安定している
    Then Jitterバッファは2フレーム以下にならない

  Scenario: Jitterバッファを手動で設定する
    When Jitterバッファを「固定: 3フレーム」に設定する
    Then Jitterバッファサイズは常に3フレームになる
    And 自動調整は行われない

  # パケットロス
  Scenario: パケットロスが発生してもFECで復元される
    Given FECが有効（冗長度10%）
    When 5%のパケットロスが発生する
    Then FECにより大部分のパケットが復元される
    And パケット復元率は90%以上になる

  Scenario: パケットロス率が高い場合
    Given FECが有効（冗長度10%）
    When 20%のパケットロスが発生する
    Then FECでは復元できないパケットが発生する
    And 補間（PLC）により急激な音の途切れを防ぐ
    And 「パケットロス率が高くなっています」と警告が表示される

  Scenario: FECが無効の場合のパケットロス
    Given FECが無効
    When 5%のパケットロスが発生する
    Then ロスしたパケットは補間（PLC）で処理される
    And 直前の音声が減衰してフェードアウトする
    And クリックノイズ（バツッという音）は発生しない

  # 遅延目標
  Scenario: LAN環境での遅延
    Given 同一LAN内の2台で接続
    And プリセット「ultra-low-latency」を使用
    And ネットワークRTTが1ms以下
    Then アプリケーション起因の片道遅延は10ms以下
    And 総片道遅延は15ms以下

  Scenario: インターネット環境での遅延
    Given インターネット越しに接続
    And プリセット「balanced」を使用
    And ネットワークRTTが50ms
    Then アプリケーション起因の片道遅延は15ms以下
    And 総片道遅延（ネットワーク込み）は約40ms

  # 帯域適応
  Scenario: 帯域が低下した場合の自動適応
    Given 帯域適応が「自動」に設定されている
    And 現在のビットレートが256kbps
    When 利用可能帯域が150kbpsに低下する
    Then ビットレートが128kbpsに自動変更される
    And 「帯域不足のため音質を下げました」と通知される
    And 遅延は増加しない

  Scenario: 帯域が回復した場合
    Given 帯域不足により128kbpsに低下済み
    When 利用可能帯域が500kbpsに回復する
    Then ビットレートが段階的に256kbpsに戻る
    And 「帯域が回復しました」と通知される

  Scenario: 帯域適応を手動に設定する
    When 帯域適応を「手動」に設定する
    And ビットレートを「128kbps」に固定する
    Then 帯域が変動してもビットレートは変更されない
    And 帯域不足時は警告のみ表示される

  # 接続品質
  Scenario: 接続品質インジケーターの表示
    Then 接続品質インジケーターが表示される
    And インジケーターは以下の状態を示す:
      | 状態 | 条件 |
      | 良好（緑） | RTT < 30ms、パケットロス < 1% |
      | 普通（黄） | RTT < 100ms、パケットロス < 5% |
      | 悪い（赤） | RTT >= 100ms または パケットロス >= 5% |

  # オーディオデバイス遅延
  Scenario: オーディオデバイスの遅延を表示する
    Given オーディオインターフェースが接続されている
    Then デバイスの入出力遅延が表示される
    And 例: 「入力: 3ms、出力: 3ms」

  Scenario: ASIO使用時の低遅延
    Given Windows環境
    And ASIO対応オーディオインターフェースが接続されている
    When ASIOドライバを選択する
    Then デバイス遅延は3ms以下になる（WASAPI: 典型的に10ms以上）
