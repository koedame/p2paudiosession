//! Connection tests based on docs/behavior/connection.feature
//!
//! Tests for session connection functionality.

use jamjam::network::{Connection, Session, SessionConfig};

/// Test: セッションを作成する
/// Given jamjamアプリケーションが起動している
/// When ユーザーが「セッション作成」を選択する
/// Then セッションが作成される
#[tokio::test]
async fn test_create_session() {
    let config = SessionConfig::default();
    let session = Session::new(config).await.expect("Failed to create session");

    // Local peer ID should be generated
    let peer_id = session.local_peer_id();
    assert!(!peer_id.is_nil(), "Peer ID should not be nil");

    // Local address should be valid
    let addr = session.local_addr();
    assert!(addr.port() > 0, "Local port should be assigned");
}

/// Test: セッション設定
/// Given デフォルトのセッション設定
/// Then max_peersは10である
#[tokio::test]
async fn test_session_config() {
    let config = SessionConfig::default();

    // Check max participants limit
    assert_eq!(config.max_peers, 10, "Default max peers should be 10");
    assert!(config.enable_mixing, "Mixing should be enabled by default");
}

/// Test: カスタムセッション設定
/// When max_peersを5に設定する
/// Then セッションは5人まで参加可能
#[tokio::test]
async fn test_custom_session_config() {
    let config = SessionConfig {
        local_port: 0,
        max_peers: 5,
        enable_mixing: true,
    };

    let session = Session::new(config).await.expect("Failed to create session");
    let peers = session.peers().await;
    assert!(peers.is_empty(), "Initial peers should be empty");
}

/// Test: 接続統計が初期化されている
#[tokio::test]
async fn test_connection_stats_initial() {
    let conn = Connection::new("127.0.0.1:0").await.expect("Failed to create connection");
    let stats = conn.stats();

    assert_eq!(stats.packets_sent, 0, "Initial packets_sent should be 0");
    assert_eq!(stats.packets_received, 0, "Initial packets_received should be 0");
    assert_eq!(stats.bytes_sent, 0, "Initial bytes_sent should be 0");
    assert_eq!(stats.bytes_received, 0, "Initial bytes_received should be 0");
}

/// Test: 接続が作成されると未接続状態
#[tokio::test]
async fn test_connection_initial_state() {
    let conn = Connection::new("127.0.0.1:0").await.expect("Failed to create connection");

    assert!(!conn.is_connected(), "New connection should not be connected");
    assert!(conn.local_addr().port() > 0, "Local address should have valid port");
}
