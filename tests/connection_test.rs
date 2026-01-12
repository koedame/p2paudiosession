//! Connection tests based on docs-spec/behavior/connection.feature
//!
//! Tests for session connection functionality.

use jamjam::network::{Connection, Session, SessionConfig};

/// Test: Create a session
/// Given jamjam application is running
/// When user selects "Create Session"
/// Then session is created
#[tokio::test]
async fn test_create_session() {
    let config = SessionConfig::default();
    let session = Session::new(config)
        .await
        .expect("Failed to create session");

    // Local peer ID should be generated
    let peer_id = session.local_peer_id();
    assert!(!peer_id.is_nil(), "Peer ID should not be nil");

    // Local address should be valid
    let addr = session.local_addr();
    assert!(addr.port() > 0, "Local port should be assigned");
}

/// Test: Session configuration
/// Given default session configuration
/// Then max_peers is 10
#[tokio::test]
async fn test_session_config() {
    let config = SessionConfig::default();

    // Check max participants limit
    assert_eq!(config.max_peers, 10, "Default max peers should be 10");
    assert!(config.enable_mixing, "Mixing should be enabled by default");
}

/// Test: Custom session configuration
/// When max_peers is set to 5
/// Then session allows up to 5 participants
#[tokio::test]
async fn test_custom_session_config() {
    let config = SessionConfig {
        local_port: 0,
        max_peers: 5,
        enable_mixing: true,
    };

    let session = Session::new(config)
        .await
        .expect("Failed to create session");
    let peers = session.peers().await;
    assert!(peers.is_empty(), "Initial peers should be empty");
}

/// Test: Connection stats are initialized
#[tokio::test]
async fn test_connection_stats_initial() {
    let conn = Connection::new("127.0.0.1:0")
        .await
        .expect("Failed to create connection");
    let stats = conn.stats();

    assert_eq!(stats.packets_sent, 0, "Initial packets_sent should be 0");
    assert_eq!(
        stats.packets_received, 0,
        "Initial packets_received should be 0"
    );
    assert_eq!(stats.bytes_sent, 0, "Initial bytes_sent should be 0");
    assert_eq!(
        stats.bytes_received, 0,
        "Initial bytes_received should be 0"
    );
}

/// Test: Connection is in disconnected state when created
#[tokio::test]
async fn test_connection_initial_state() {
    let conn = Connection::new("127.0.0.1:0")
        .await
        .expect("Failed to create connection");

    assert!(
        !conn.is_connected(),
        "New connection should not be connected"
    );
    assert!(
        conn.local_addr().port() > 0,
        "Local address should have valid port"
    );
}

/// Test: Session can be recreated on same port after stop
/// This verifies SO_REUSEADDR is working correctly
#[tokio::test]
async fn test_session_recreate_same_port() {
    // Create first session with auto-assigned port
    let config1 = SessionConfig::default();
    let mut session1 = Session::new(config1)
        .await
        .expect("Failed to create first session");
    session1.start();

    let port = session1.local_addr().port();

    // Stop and drop the first session
    session1.stop();
    drop(session1);

    // Small delay to ensure cleanup
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Create second session on the same port
    let config2 = SessionConfig {
        local_port: port,
        max_peers: 10,
        enable_mixing: true,
    };

    let session2 = Session::new(config2).await;
    assert!(
        session2.is_ok(),
        "Should be able to create session on same port after stop: {:?}",
        session2.err()
    );
    assert_eq!(session2.unwrap().local_addr().port(), port);
}

/// Test: Multiple session create/stop cycles on same port
#[tokio::test]
async fn test_session_repeated_recreate() {
    // Get an available port
    let config = SessionConfig::default();
    let session = Session::new(config)
        .await
        .expect("Failed to create initial session");
    let port = session.local_addr().port();
    drop(session);

    // Perform multiple create/stop cycles
    for i in 0..5 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let config = SessionConfig {
            local_port: port,
            max_peers: 10,
            enable_mixing: true,
        };

        let session = Session::new(config).await;
        assert!(
            session.is_ok(),
            "Session creation cycle {} should succeed: {:?}",
            i + 1,
            session.err()
        );

        let mut s = session.unwrap();
        s.start();
        s.stop();
        drop(s);
    }
}
