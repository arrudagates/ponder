#![deny(unsafe_code)]
//! Basic Implementation of MQTT Server
//!
//! The basic implementation of MQTT proxy, supporting v3.1.1 and v5.0 protocols, with TLS and
//! WebSocket functionality.
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use rmqtt_net::{Builder, ListenerType};
//! use std::net::SocketAddr;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let builder = Builder::new()
//!         .name("MyBroker")
//!         .laddr("127.0.0.1:1883".parse()?);
//!
//!     let listener = builder.bind()?;
//!     loop {
//!         let acceptor = listener.accept().await?;
//!         let dispatcher = acceptor.tcp()?;
//!         // Handle connection...
//!     }
//!     Ok(())
//! }
//! ```

mod builder;
mod error;
mod stream;

/// Server configuration and listener management
pub use builder::{Builder, Listener, ListenerType};

/// Error types for MQTT operations
pub use error::MqttError;

/// MQTT protocol implementations and stream handling
pub use stream::{v3, v5, MqttStream};

/// Convenience type alias for generic errors
pub type Error = anyhow::Error;

/// Result type alias using crate's Error type
pub type Result<T> = anyhow::Result<T, Error>;
