//! AES-256-GCM encrypted state management for Ferrum.
//!
//! Secrets are never written to disk in plain text. All sensitive values
//! are encrypted at rest inside `ferrum.fstate`.

mod crypto;
mod error;
mod resource;
mod state_file;

pub use crypto::{derive_key_from_passphrase, generate_key, EncryptedBlob};
pub use error::{Result, StateError};
pub use resource::{ResourceInstance, ResourceStatus, SecretValue};
pub use state_file::{State, StateMetadata, STATE_FILENAME, KEY_FILENAME};
