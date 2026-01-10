Feature: 国際化
  jamjamはユーザーインターフェースの多言語表示に対応する

  Background:
    Given jamjamアプリケーションがインストールされている

  Scenario: 初回起動時のシステム言語検出
    Given システムロケールが日本語（ja）に設定されている
    And config.tomlにlanguage設定が存在しない
    When ユーザーがjamjamを初めて起動する
    Then UIが日本語で表示される

  Scenario: 設定画面での言語切替
    Given UIが英語で表示されている
    When ユーザーが言語設定で「日本語」を選択する
    Then UIが即座に日本語で表示される
    And config.tomlのlanguageが"ja"に更新される

  Scenario: 翻訳キーが存在しない場合のフォールバック
    Given UI言語が日本語に設定されている
    And 翻訳キー"experimental.new_feature"がja.jsonに存在しない
    When UIがそのキーを表示しようとする
    Then 英語の翻訳が表示される
    And コンソールに警告が出力される

  Scenario: 言語設定の永続化
    Given ユーザーが言語設定を日本語に変更している
    When ユーザーがjamjamを終了して再起動する
    Then UIが日本語で表示される
