pub mod proto {
    tonic::include_proto!("ferrum.provider.v1");
}

mod client;
mod error;

pub use client::ProviderBridgeClient;
pub use error::{BridgeError, Result};
