pub mod proto {
    tonic::include_proto!("ferrum.provider.v1");
}

/// Official HashiCorp Terraform Plugin Protocol v5 (from hashicorp/terraform).
pub mod tfplugin5 {
    tonic::include_proto!("tfplugin5");
}

/// Official HashiCorp Terraform Plugin Protocol v6 (from hashicorp/terraform).
pub mod tfplugin6 {
    tonic::include_proto!("tfplugin6");
}

mod client;
mod error;
mod executor;
mod plugin;
mod pool;
mod schema;
mod tfplugin;

pub use client::ProviderBridgeClient;
pub use error::{BridgeError, Result};
pub use executor::{provider_apply_resource, provider_plan_resource, ApplyResult};
pub use plugin::{
    default_plugins_dir, find_provider, launch_and_handshake, preflight_security_check,
    InstalledProvider, PluginManager, ProviderSpec, OFFICIAL_PROVIDERS,
};
pub use pool::{provider_key_for_resource, warm_pool_for_resources, ProviderPool};
pub use schema::{
    fetch_provider_schemas, provider_for_resource_type, ProviderSchemaRegistry, ResourceSchema,
};
pub use tfplugin::{dynamic_to_json, json_to_dynamic, PluginProtocol, TfPluginClient};
