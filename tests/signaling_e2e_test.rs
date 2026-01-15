//! E2E tests for signaling server connection flow
//!
//! Tests the full flow: connect → create/join room → disconnect
//! Also covers error cases like timeout and connection refused.

use std::net::TcpListener;
use std::time::Duration;

use jamjam::network::{
    generate_invite_code, is_invite_code_format, SignalingClient, SignalingMessage, SignalingServer,
};

/// Find an available port for testing
fn find_available_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("Failed to bind to ephemeral port")
        .local_addr()
        .expect("Failed to get local address")
        .port()
}

/// Helper to start a signaling server in background
async fn start_test_server(port: u16) -> tokio::task::JoinHandle<()> {
    let addr = format!("127.0.0.1:{}", port);
    let server = SignalingServer::new();

    tokio::spawn(async move {
        // Server runs until cancelled
        let _ = server.run(&addr).await;
    })
}

/// Test: Connect to signaling server
/// Given a running signaling server
/// When client connects
/// Then connection is established
#[tokio::test]
async fn test_connect_to_signaling_server() {
    let port = find_available_port();
    let server_handle = start_test_server(port).await;

    // Wait for server to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = SignalingClient::new(&format!("ws://127.0.0.1:{}", port));
    let result = client.connect().await;

    // Clean up
    server_handle.abort();

    assert!(result.is_ok(), "Should connect to signaling server");
}

/// Test: Create room via signaling server
/// Given a connected client
/// When client creates a room
/// Then room is created with invite code
#[tokio::test]
async fn test_create_room() {
    let port = find_available_port();
    let server_handle = start_test_server(port).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = SignalingClient::new(&format!("ws://127.0.0.1:{}", port));
    let mut conn = client.connect().await.expect("Failed to connect");

    // Create room
    conn.send(SignalingMessage::CreateRoom {
        room_name: "Test Room".to_string(),
        password: None,
        peer_name: "Host".to_string(),
    })
    .await
    .expect("Failed to send create room");

    let response = conn.recv().await.expect("Failed to receive response");

    // Clean up
    let _ = conn.close().await;
    server_handle.abort();

    match response {
        SignalingMessage::RoomCreated {
            room_id,
            peer_id,
            invite_code,
        } => {
            assert!(!room_id.is_empty(), "Room ID should not be empty");
            assert!(!peer_id.is_nil(), "Peer ID should not be nil");
            assert!(
                is_invite_code_format(&invite_code),
                "Invite code '{}' should be valid format",
                invite_code
            );
        }
        other => panic!("Expected RoomCreated, got {:?}", other),
    }
}

/// Test: Join room via invite code
/// Given a created room
/// When another client joins via invite code
/// Then client joins successfully
#[tokio::test]
async fn test_join_room_via_invite_code() {
    let port = find_available_port();
    let server_handle = start_test_server(port).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Client 1: Create room
    let client1 = SignalingClient::new(&format!("ws://127.0.0.1:{}", port));
    let mut conn1 = client1.connect().await.expect("Failed to connect client 1");

    conn1
        .send(SignalingMessage::CreateRoom {
            room_name: "Test Room".to_string(),
            password: None,
            peer_name: "Host".to_string(),
        })
        .await
        .expect("Failed to send create room");

    let create_response = conn1
        .recv()
        .await
        .expect("Failed to receive create response");
    let invite_code = match create_response {
        SignalingMessage::RoomCreated { invite_code, .. } => invite_code,
        other => panic!("Expected RoomCreated, got {:?}", other),
    };

    // Client 2: Join room via invite code
    let client2 = SignalingClient::new(&format!("ws://127.0.0.1:{}", port));
    let mut conn2 = client2.connect().await.expect("Failed to connect client 2");

    conn2
        .send(SignalingMessage::JoinRoom {
            room_id: invite_code.clone(), // Use invite code as room_id
            password: None,
            peer_name: "Guest".to_string(),
        })
        .await
        .expect("Failed to send join room");

    let join_response = conn2.recv().await.expect("Failed to receive join response");

    // Clean up
    let _ = conn1.close().await;
    let _ = conn2.close().await;
    server_handle.abort();

    match join_response {
        SignalingMessage::RoomJoined {
            room_id: _,
            peer_id,
            peers,
        } => {
            assert!(!peer_id.is_nil(), "Peer ID should not be nil");
            assert_eq!(peers.len(), 1, "Should see host peer");
            assert_eq!(peers[0].name, "Host", "Host name should match");
        }
        other => panic!("Expected RoomJoined, got {:?}", other),
    }
}

/// Test: Full flow - create room, join, leave, disconnect
/// Given two clients
/// When they go through full session flow
/// Then all operations complete successfully
#[tokio::test]
async fn test_full_session_flow() {
    let port = find_available_port();
    let server_handle = start_test_server(port).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Step 1: Host creates room
    let client1 = SignalingClient::new(&format!("ws://127.0.0.1:{}", port));
    let mut conn1 = client1.connect().await.expect("Failed to connect host");

    conn1
        .send(SignalingMessage::CreateRoom {
            room_name: "Session Room".to_string(),
            password: None,
            peer_name: "Host".to_string(),
        })
        .await
        .expect("Failed to create room");

    let create_response = conn1
        .recv()
        .await
        .expect("Failed to receive create response");
    let invite_code = match create_response {
        SignalingMessage::RoomCreated { invite_code, .. } => invite_code,
        other => panic!("Expected RoomCreated, got {:?}", other),
    };

    // Step 2: Guest joins room
    let client2 = SignalingClient::new(&format!("ws://127.0.0.1:{}", port));
    let mut conn2 = client2.connect().await.expect("Failed to connect guest");

    conn2
        .send(SignalingMessage::JoinRoom {
            room_id: invite_code,
            password: None,
            peer_name: "Guest".to_string(),
        })
        .await
        .expect("Failed to join room");

    let join_response = conn2.recv().await.expect("Failed to receive join response");
    assert!(
        matches!(join_response, SignalingMessage::RoomJoined { .. }),
        "Guest should join successfully"
    );

    // Step 3: Host receives peer joined notification
    let peer_joined = conn1.recv().await.expect("Failed to receive peer joined");
    match peer_joined {
        SignalingMessage::PeerJoined { peer } => {
            assert_eq!(peer.name, "Guest", "Should receive guest join notification");
        }
        other => panic!("Expected PeerJoined, got {:?}", other),
    }

    // Step 4: Guest leaves room
    conn2
        .send(SignalingMessage::LeaveRoom)
        .await
        .expect("Failed to send leave room");

    // Step 5: Host receives peer left notification
    let peer_left = conn1.recv().await.expect("Failed to receive peer left");
    match peer_left {
        SignalingMessage::PeerLeft { peer_id: _ } => {
            // Guest has left
        }
        other => panic!("Expected PeerLeft, got {:?}", other),
    }

    // Step 6: Clean disconnect
    let _ = conn1.close().await;
    let _ = conn2.close().await;
    server_handle.abort();
}

/// Test: Connection timeout when server is not available
/// Given no server running
/// When client tries to connect with timeout
/// Then connection fails with timeout error
#[tokio::test]
async fn test_connection_timeout() {
    // Use a port that is definitely not running a server
    let port = find_available_port();
    let client = SignalingClient::new(&format!("ws://127.0.0.1:{}", port));

    // Try to connect with a timeout
    let result = tokio::time::timeout(Duration::from_secs(2), client.connect()).await;

    match result {
        Ok(conn_result) => {
            // Connection attempt completed (with error)
            assert!(
                conn_result.is_err(),
                "Should fail to connect to non-existent server"
            );
        }
        Err(_) => {
            // Timeout occurred - this is also acceptable
        }
    }
}

/// Test: Join non-existent room
/// Given a connected client
/// When client tries to join a non-existent room
/// Then error is returned
#[tokio::test]
async fn test_join_nonexistent_room() {
    let port = find_available_port();
    let server_handle = start_test_server(port).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client = SignalingClient::new(&format!("ws://127.0.0.1:{}", port));
    let mut conn = client.connect().await.expect("Failed to connect");

    conn.send(SignalingMessage::JoinRoom {
        room_id: "NONEXISTENT".to_string(),
        password: None,
        peer_name: "Guest".to_string(),
    })
    .await
    .expect("Failed to send join room");

    let response = conn.recv().await.expect("Failed to receive response");

    // Clean up
    let _ = conn.close().await;
    server_handle.abort();

    match response {
        SignalingMessage::Error { message } => {
            assert!(
                message.contains("not found"),
                "Error should indicate room not found"
            );
        }
        other => panic!("Expected Error, got {:?}", other),
    }
}

/// Test: Join room with wrong password
/// Given a password-protected room
/// When client tries to join with wrong password
/// Then access is denied
#[tokio::test]
async fn test_join_room_wrong_password() {
    let port = find_available_port();
    let server_handle = start_test_server(port).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create room with password
    let client1 = SignalingClient::new(&format!("ws://127.0.0.1:{}", port));
    let mut conn1 = client1.connect().await.expect("Failed to connect host");

    conn1
        .send(SignalingMessage::CreateRoom {
            room_name: "Secure Room".to_string(),
            password: Some("secret123".to_string()),
            peer_name: "Host".to_string(),
        })
        .await
        .expect("Failed to create room");

    let create_response = conn1
        .recv()
        .await
        .expect("Failed to receive create response");
    let invite_code = match create_response {
        SignalingMessage::RoomCreated { invite_code, .. } => invite_code,
        other => panic!("Expected RoomCreated, got {:?}", other),
    };

    // Try to join with wrong password
    let client2 = SignalingClient::new(&format!("ws://127.0.0.1:{}", port));
    let mut conn2 = client2.connect().await.expect("Failed to connect guest");

    conn2
        .send(SignalingMessage::JoinRoom {
            room_id: invite_code,
            password: Some("wrongpassword".to_string()),
            peer_name: "Guest".to_string(),
        })
        .await
        .expect("Failed to send join room");

    let response = conn2.recv().await.expect("Failed to receive response");

    // Clean up
    let _ = conn1.close().await;
    let _ = conn2.close().await;
    server_handle.abort();

    match response {
        SignalingMessage::Error { message } => {
            assert!(
                message.contains("password") || message.contains("Invalid"),
                "Error should indicate invalid password"
            );
        }
        other => panic!("Expected Error, got {:?}", other),
    }
}

/// Test: List rooms
/// Given multiple rooms exist
/// When client lists rooms
/// Then all rooms are returned
#[tokio::test]
async fn test_list_rooms() {
    let port = find_available_port();
    let server_handle = start_test_server(port).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create two rooms
    let client1 = SignalingClient::new(&format!("ws://127.0.0.1:{}", port));
    let mut conn1 = client1.connect().await.expect("Failed to connect");

    conn1
        .send(SignalingMessage::CreateRoom {
            room_name: "Room 1".to_string(),
            password: None,
            peer_name: "Host1".to_string(),
        })
        .await
        .unwrap();
    let _ = conn1.recv().await.unwrap();

    let client2 = SignalingClient::new(&format!("ws://127.0.0.1:{}", port));
    let mut conn2 = client2.connect().await.expect("Failed to connect");

    conn2
        .send(SignalingMessage::CreateRoom {
            room_name: "Room 2".to_string(),
            password: Some("secret".to_string()),
            peer_name: "Host2".to_string(),
        })
        .await
        .unwrap();
    let _ = conn2.recv().await.unwrap();

    // List rooms from third client
    let client3 = SignalingClient::new(&format!("ws://127.0.0.1:{}", port));
    let mut conn3 = client3.connect().await.expect("Failed to connect");

    conn3
        .send(SignalingMessage::ListRooms)
        .await
        .expect("Failed to list rooms");

    let response = conn3.recv().await.expect("Failed to receive room list");

    // Clean up
    let _ = conn1.close().await;
    let _ = conn2.close().await;
    let _ = conn3.close().await;
    server_handle.abort();

    match response {
        SignalingMessage::RoomList { rooms } => {
            assert_eq!(rooms.len(), 2, "Should have 2 rooms");

            let room1 = rooms.iter().find(|r| r.name == "Room 1");
            assert!(room1.is_some(), "Room 1 should exist");
            assert!(
                !room1.unwrap().has_password,
                "Room 1 should not have password"
            );

            let room2 = rooms.iter().find(|r| r.name == "Room 2");
            assert!(room2.is_some(), "Room 2 should exist");
            assert!(room2.unwrap().has_password, "Room 2 should have password");
        }
        other => panic!("Expected RoomList, got {:?}", other),
    }
}

/// Test: Room cleanup after all peers leave
/// Given a room with one peer
/// When peer disconnects
/// Then room is cleaned up
#[tokio::test]
async fn test_room_cleanup_on_disconnect() {
    let port = find_available_port();
    let server_handle = start_test_server(port).await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create room
    let client1 = SignalingClient::new(&format!("ws://127.0.0.1:{}", port));
    let mut conn1 = client1.connect().await.expect("Failed to connect");

    conn1
        .send(SignalingMessage::CreateRoom {
            room_name: "Temp Room".to_string(),
            password: None,
            peer_name: "Host".to_string(),
        })
        .await
        .unwrap();
    let _ = conn1.recv().await.unwrap();

    // Disconnect by closing connection
    let _ = conn1.close().await;

    // Wait for cleanup
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check room is removed
    let client2 = SignalingClient::new(&format!("ws://127.0.0.1:{}", port));
    let mut conn2 = client2.connect().await.expect("Failed to connect");

    conn2
        .send(SignalingMessage::ListRooms)
        .await
        .expect("Failed to list rooms");

    let response = conn2.recv().await.expect("Failed to receive room list");

    // Clean up
    let _ = conn2.close().await;
    server_handle.abort();

    match response {
        SignalingMessage::RoomList { rooms } => {
            assert!(
                rooms.is_empty(),
                "Room should be cleaned up after host disconnect"
            );
        }
        other => panic!("Expected RoomList, got {:?}", other),
    }
}

/// Test: Invite code format validation
#[test]
fn test_invite_code_format() {
    // Valid codes
    assert!(is_invite_code_format("ABC234"));
    assert!(is_invite_code_format("HJKMNP"));

    // Invalid codes
    assert!(!is_invite_code_format("ABC23")); // Too short
    assert!(!is_invite_code_format("abc234")); // Lowercase
    assert!(!is_invite_code_format("ABC230")); // Contains '0'
    assert!(!is_invite_code_format("ABCDEO")); // Contains 'O'

    // Generated codes should always be valid
    for _ in 0..100 {
        let code = generate_invite_code();
        assert!(
            is_invite_code_format(&code),
            "Generated code '{}' should be valid",
            code
        );
    }
}
