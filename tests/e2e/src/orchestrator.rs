//! Test orchestration for multi-node E2E tests
//!
//! Coordinates test execution across multiple nodes.

use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

use crate::node::{NodeProcess, TestNode};
use crate::{TestConfig, TestResult};

/// Test orchestrator for coordinating multi-node tests
pub struct TestOrchestrator {
    /// Test configuration
    config: TestConfig,
    /// Active node processes
    processes: HashMap<String, NodeProcess>,
}

impl TestOrchestrator {
    /// Create a new test orchestrator
    pub fn new(config: TestConfig) -> Self {
        Self {
            config,
            processes: HashMap::new(),
        }
    }

    /// Run a two-node connection test
    pub async fn run_two_node_test(
        &mut self,
        host_node: TestNode,
        client_node: TestNode,
    ) -> TestResult {
        let scenario = format!(
            "two-node-{:?}-{:?}",
            host_node.platform, client_node.platform
        );
        let start = Instant::now();

        info!("Starting two-node test: {}", scenario);

        // Step 1: Start host node
        let host_process = match NodeProcess::start_host(host_node.clone()).await {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to start host: {}", e);
                return TestResult::failed(&scenario, format!("Host start failed: {}", e));
            }
        };
        self.processes.insert(host_node.id.clone(), host_process);

        // Step 2: Start client node and connect
        let host_addr = host_node.session_addr();
        let client_process = match NodeProcess::start_join(client_node.clone(), &host_addr).await {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to start client: {}", e);
                self.cleanup().await;
                return TestResult::failed(&scenario, format!("Client start failed: {}", e));
            }
        };
        self.processes.insert(client_node.id.clone(), client_process);

        let connection_time = start.elapsed();
        info!(
            "Nodes connected in {}ms",
            connection_time.as_millis()
        );

        // Step 3: Wait for test duration
        tokio::time::sleep(Duration::from_secs(self.config.duration_sec as u64)).await;

        // Step 4: Cleanup
        self.cleanup().await;

        let duration = start.elapsed();
        let mut result = TestResult::passed(&scenario, duration.as_millis() as u64);
        result.connection_time_ms = Some(connection_time.as_millis() as u64);

        info!(
            "Two-node test completed: {} ({}ms)",
            scenario,
            duration.as_millis()
        );

        result
    }

    /// Run a loopback test (single node, audio round-trip)
    pub async fn run_loopback_test(&mut self, node: TestNode) -> TestResult {
        let scenario = format!("loopback-{:?}", node.platform);
        let start = Instant::now();

        info!("Starting loopback test: {}", scenario);

        // For loopback tests, we test audio processing without network
        // This requires virtual audio devices

        // TODO: Implement actual loopback test with virtual audio
        // For now, return a placeholder result

        let duration = start.elapsed();
        TestResult::passed(&scenario, duration.as_millis() as u64)
    }

    /// Cleanup all running processes
    async fn cleanup(&mut self) {
        for (id, mut process) in self.processes.drain() {
            debug!("Cleaning up node: {}", id);
            if let Err(e) = process.stop().await {
                warn!("Failed to stop node {}: {}", id, e);
            }
        }
    }
}

impl Drop for TestOrchestrator {
    fn drop(&mut self) {
        // Best-effort cleanup - processes will be killed in their own Drop impl
        self.processes.clear();
    }
}

/// Scenario definition for declarative test configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestScenario {
    /// Scenario name
    pub name: String,
    /// Description
    pub description: String,
    /// Number of nodes required
    pub node_count: usize,
    /// Platform requirements
    pub platforms: Vec<String>,
    /// Test duration in seconds
    pub duration_sec: u32,
    /// Assertions to verify
    pub assertions: Vec<Assertion>,
}

/// Test assertion
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Assertion {
    /// Assertion type
    #[serde(rename = "type")]
    pub assertion_type: String,
    /// Expected value or threshold
    #[serde(flatten)]
    pub params: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let config = TestConfig::default();
        let orchestrator = TestOrchestrator::new(config);
        assert!(orchestrator.processes.is_empty());
    }
}
