//! E2E test scenarios
//!
//! This module contains end-to-end test scenarios for various configurations.

#[cfg(feature = "loopback")]
pub mod loopback;

#[cfg(feature = "network-local")]
pub mod two_node;

#[cfg(feature = "remote")]
pub mod cross_platform;

#[cfg(feature = "remote")]
pub mod eight_node;
