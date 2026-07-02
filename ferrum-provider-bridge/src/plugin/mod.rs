mod checksum;
mod handshake;
mod manager;
mod registry;

pub use handshake::{launch_and_handshake, HandshakeResult};
pub use manager::{
    default_plugins_dir, preflight_security_check, InstalledProvider, PluginManager,
};
pub use registry::{find_provider, ProviderSpec, OFFICIAL_PROVIDERS};
