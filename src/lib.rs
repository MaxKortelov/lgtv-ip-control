//! Async Rust client for controlling LG TVs over IP Control.
//!
//! # Example
//!
//! ```no_run
//! use lgtv_ip_control::{Apps, LGTV};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut tv = LGTV::connect_tcp(
//!         "IP_ADDRESS",
//!         "MAC_ADDRESS",
//!         Some("KEY_CODE"),
//!     )
//!     .await?;
//!
//!     tv.launch_app(Apps::Youtube).await?;
//!
//!     Ok(())
//! }
//! ```
pub mod constants;
mod encryption;
pub mod lg_tv;
mod tcp_connection;

pub use constants::types::{AppDetails, Apps, ConnectionType, PowerStates};
pub use lg_tv::LGTV;
