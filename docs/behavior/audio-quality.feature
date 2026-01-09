Feature: 音声品質
  jamjamでの音声品質に関する振る舞い

  Background:
    Given jamjamアプリケーションが起動している
    And オーディオデバイスが正常に認識されている
    And セッションに接続済み

  # コーデック選択
  Scenario: 非圧縮PCMコーデックを使用する
    Given ネットワーク帯域が10Mbps以上
    When コーデックを「非圧縮PCM」に設定する
    Then 音声は圧縮されずに送信される
    And 送信ビットレートは約1.5Mbps/chになる
    And コーデック起因の遅延は0msになる

  Scenario: Opusコーデックを使用する
    Given ネットワーク帯域が1Mbps程度
    When コーデックを「Opus 128kbps」に設定する
    Then 音声はOpusで圧縮されて送信される
    And 送信ビットレートは約128kbpsになる

  Scenario: 帯域不足でコーデックが自動変更される
    Given 帯域適応が「自動」に設定されている
    And コーデックが「Opus 256kbps」に設定されている
    When 利用可能帯域が100kbpsに低下する
    Then コーデックが「Opus 64kbps」に自動変更される
    And ユーザーに「帯域不足のため音質を下げました」と通知される

  # サンプルレート
  Scenario: サンプルレート48kHzで動作する
    When サンプルレートを「48000Hz」に設定する
    Then オーディオエンジンは48kHzで動作する
    And 相手にも48kHzで音声が伝送される

  Scenario: サンプルレート96kHzで動作する
    Given オーディオインターフェースが96kHz対応
    When サンプルレートを「96000Hz」に設定する
    Then オーディオエンジンは96kHzで動作する
    And 相手にも96kHzで音声が伝送される

  Scenario: 異なるサンプルレートの参加者がいる場合
    Given ホストが48kHzに設定している
    And 参加者Aが96kHzに設定している
    When セッションが開始される
    Then 参加者Aの音声は48kHzにリサンプリングされる
    And ホストの音声は参加者Aに48kHzで送信される

  # チャンネル
  Scenario: モノラル入力で動作する
    When 入力チャンネルを「モノラル」に設定する
    Then 1チャンネルの音声が送信される
    And 受信側ではモノラルまたは両チャンネル同一で再生される

  Scenario: ステレオ入力で動作する
    When 入力チャンネルを「ステレオ」に設定する
    Then 2チャンネルの音声が送信される
    And 受信側ではステレオで再生される

  # フレームサイズ
  Scenario: フレームサイズ64サンプルで動作する
    When フレームサイズを「64 samples」に設定する
    Then オーディオバッファは64サンプル（約1.33ms @ 48kHz）になる
    And フレームサイズ起因の遅延は約1.33msになる

  Scenario: フレームサイズ256サンプルで動作する
    When フレームサイズを「256 samples」に設定する
    Then オーディオバッファは256サンプル（約5.33ms @ 48kHz）になる
    And バッファアンダーランが発生しにくくなる

  # ローカルモニタリング
  Scenario: ローカルモニタリングを有効にする
    When ローカルモニタリングを「ON」に設定する
    Then 自分の音声がネットワーク遅延なしで聞こえる
    And 他の参加者の音声も同時に聞こえる

  Scenario: ローカルモニタリングを無効にする
    When ローカルモニタリングを「OFF」に設定する
    Then 自分の音声は直接聞こえない
    And 他の参加者の音声のみ聞こえる

  # 音声処理なし
  Scenario: 音声がピュアに伝送される
    Given 音声処理（AEC、NS、AGC）が無効
    When 楽器（ギター）を演奏する
    Then 音声は一切の処理なしで伝送される
    And 受信側では演奏したままの音が聞こえる

  # プリセット
  Scenario: ultra-low-latencyプリセットを使用する
    When プリセット「ultra-low-latency」を選択する
    Then コーデックが「非圧縮PCM」に設定される
    And フレームサイズが「64 samples」に設定される
    And Jitterバッファが「最小（1フレーム）」に設定される
    And FECが「OFF」に設定される

  Scenario: balancedプリセットを使用する
    When プリセット「balanced」を選択する
    Then コーデックが「Opus 128kbps」に設定される
    And フレームサイズが「128 samples」に設定される
    And Jitterバッファが「4フレーム」に設定される
    And FECが「ON（10%）」に設定される

  Scenario: high-qualityプリセットを使用する
    When プリセット「high-quality」を選択する
    Then コーデックが「Opus 256kbps」に設定される
    And フレームサイズが「256 samples」に設定される
    And Jitterバッファが「8フレーム」に設定される
    And FECが「ON（20%）」に設定される

  Scenario: カスタムプリセットを保存する
    Given プリセット「balanced」をベースに設定を変更済み
    When 「プリセットを保存」を選択する
    And プリセット名「my-setting」を入力する
    Then カスタムプリセット「my-setting」が保存される
    And 次回以降「my-setting」がプリセット一覧に表示される
