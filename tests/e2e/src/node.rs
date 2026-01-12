//! Remote test node management
//!
//! Manages test nodes (local or remote machines) for E2E testing.

use std::process::Stdio;
use tokio::process::{Child, Command};
use tracing::{debug, info};

/// Supported platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
}

impl Platform {
    /// Detect the current platform
    pub fn current() -> Self {
        #[cfg(target_os = "linux")]
        return Platform::Linux;
        #[cfg(target_os = "macos")]
        return Platform::MacOS;
        #[cfg(target_os = "windows")]
        return Platform::Windows;
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        panic!("Unsupported platform");
    }

    /// Get the virtual audio setup script name
    pub fn virtual_audio_script(&self) -> &'static str {
        match self {
            Platform::Linux => "setup-virtual-audio-linux.sh",
            Platform::MacOS => "setup-virtual-audio-macos.sh",
            Platform::Windows => "setup-virtual-audio-windows.ps1",
        }
    }

    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::Linux => "linux",
            Platform::MacOS => "macos",
            Platform::Windows => "windows",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "linux" => Platform::Linux,
            "macos" | "darwin" => Platform::MacOS,
            "windows" | "win" => Platform::Windows,
            _ => Platform::Linux, // Default to Linux
        }
    }
}

/// A test node (local or remote machine)
#[derive(Debug, Clone)]
pub struct TestNode {
    /// Unique identifier for this node
    pub id: String,
    /// Platform of this node
    pub platform: Platform,
    /// Address for SSH connection (None for local)
    pub ssh_address: Option<String>,
    /// Path to jamjam binary on this node
    pub binary_path: String,
    /// Port for jamjam session
    pub session_port: u16,
}

impl TestNode {
    /// Create a local test node with default settings
    pub fn local(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            platform: Platform::current(),
            ssh_address: None,
            binary_path: "target/release/jamjam".to_string(),
            session_port: 0, // Will be assigned
        }
    }

    /// Create a local test node with specific settings
    pub fn local_with_config(id: impl Into<String>, binary_path: impl Into<String>, port: u16) -> Self {
        Self {
            id: id.into(),
            platform: Platform::current(),
            ssh_address: None,
            binary_path: binary_path.into(),
            session_port: port,
        }
    }

    /// Get the platform of this node
    pub fn platform(&self) -> &Platform {
        &self.platform
    }

    /// Create a remote test node
    pub fn remote(
        id: impl Into<String>,
        platform: Platform,
        ssh_address: impl Into<String>,
        binary_path: impl Into<String>,
        port: u16,
    ) -> Self {
        Self {
            id: id.into(),
            platform,
            ssh_address: Some(ssh_address.into()),
            binary_path: binary_path.into(),
            session_port: port,
        }
    }

    /// Check if this is a local node
    pub fn is_local(&self) -> bool {
        self.ssh_address.is_none()
    }

    /// Get the session address for this node
    pub fn session_addr(&self) -> String {
        if self.is_local() {
            format!("127.0.0.1:{}", self.session_port)
        } else {
            // Extract host from SSH address
            let host = self
                .ssh_address
                .as_ref()
                .map(|s| s.split('@').last().unwrap_or(s))
                .unwrap_or("127.0.0.1");
            format!("{}:{}", host, self.session_port)
        }
    }
}

/// Handle to a running jamjam process
pub struct NodeProcess {
    /// The node this process belongs to
    pub node: TestNode,
    /// The child process (only for local nodes)
    child: Option<Child>,
    /// Whether the process is running
    running: bool,
}

impl NodeProcess {
    /// Start jamjam on a local node as host
    pub async fn start_host(node: TestNode) -> Result<Self, NodeError> {
        if !node.is_local() {
            return Err(NodeError::RemoteNotSupported);
        }

        info!("Starting jamjam host on node {} port {}", node.id, node.session_port);

        let child = Command::new(&node.binary_path)
            .arg("host")
            .arg("--port")
            .arg(node.session_port.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| NodeError::SpawnFailed(e.to_string()))?;

        // Wait a bit for the process to start
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        Ok(Self {
            node,
            child: Some(child),
            running: true,
        })
    }

    /// Start jamjam on a local node and join a session
    pub async fn start_join(node: TestNode, host_addr: &str) -> Result<Self, NodeError> {
        if !node.is_local() {
            return Err(NodeError::RemoteNotSupported);
        }

        info!("Starting jamjam join on node {} to {}", node.id, host_addr);

        let child = Command::new(&node.binary_path)
            .arg("join")
            .arg(host_addr)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| NodeError::SpawnFailed(e.to_string()))?;

        // Wait a bit for the process to start
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        Ok(Self {
            node,
            child: Some(child),
            running: true,
        })
    }

    /// Stop the process
    pub async fn stop(&mut self) -> Result<(), NodeError> {
        if let Some(ref mut child) = self.child {
            debug!("Stopping node {}", self.node.id);
            child.kill().await.ok();
            self.running = false;
        }
        Ok(())
    }

    /// Check if the process is still running
    pub fn is_running(&self) -> bool {
        self.running
    }
}

impl Drop for NodeProcess {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            // Try to kill the process synchronously
            // This is a best-effort cleanup
            let _ = child.start_kill();
        }
    }
}

/// Errors that can occur during node operations
#[derive(Debug, thiserror::Error)]
pub enum NodeError {
    #[error("Failed to spawn process: {0}")]
    SpawnFailed(String),

    #[error("Remote node operations not yet supported")]
    RemoteNotSupported,

    #[error("Node not running")]
    NotRunning,

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = Platform::current();
        // Should not panic
        assert!(matches!(
            platform,
            Platform::Linux | Platform::MacOS | Platform::Windows
        ));
    }

    #[test]
    fn test_local_node_creation() {
        let node = TestNode::local_with_config("test-node", "/usr/bin/jamjam", 5000);
        assert!(node.is_local());
        assert_eq!(node.session_addr(), "127.0.0.1:5000");
    }

    #[test]
    fn test_local_node_simple() {
        let node = TestNode::local("simple-node");
        assert!(node.is_local());
        assert_eq!(node.binary_path, "target/release/jamjam");
    }

    #[test]
    fn test_remote_node_creation() {
        let node = TestNode::remote(
            "remote-node",
            Platform::Linux,
            "user@192.168.1.100",
            "/home/user/jamjam",
            5001,
        );
        assert!(!node.is_local());
        assert_eq!(node.session_addr(), "192.168.1.100:5001");
    }
}
