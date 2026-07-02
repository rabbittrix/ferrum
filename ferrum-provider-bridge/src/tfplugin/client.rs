//! Terraform Plugin Protocol gRPC client (official HashiCorp v5 / v6 protos).

use std::path::Path;
use std::sync::Arc;

use tokio::process::Child;
use tokio::sync::Mutex;
use tonic::transport::Channel;

use crate::error::{BridgeError, Result};
use crate::plugin::{launch_and_handshake, preflight_security_check, HandshakeResult};
use crate::tfplugin5::provider_client::ProviderClient as ProviderClientV5;
use crate::tfplugin6::provider_client::ProviderClient as ProviderClientV6;

pub fn json_to_dynamic_v5(value: &serde_json::Value) -> Result<crate::tfplugin5::DynamicValue> {
    let mut buf = Vec::new();
    rmp_serde::encode::write(&mut buf, value)
        .map_err(|e| BridgeError::Provider(format!("msgpack encode: {e}")))?;
    Ok(crate::tfplugin5::DynamicValue {
        msgpack: buf,
        json: vec![],
    })
}

pub fn json_to_dynamic_v6(value: &serde_json::Value) -> Result<crate::tfplugin6::DynamicValue> {
    let mut buf = Vec::new();
    rmp_serde::encode::write(&mut buf, value)
        .map_err(|e| BridgeError::Provider(format!("msgpack encode: {e}")))?;
    Ok(crate::tfplugin6::DynamicValue {
        msgpack: buf,
        json: vec![],
    })
}

pub fn dynamic_to_json_v5(value: &crate::tfplugin5::DynamicValue) -> Result<serde_json::Value> {
    decode_dynamic(&value.msgpack, &value.json)
}

pub fn dynamic_to_json_v6(value: &crate::tfplugin6::DynamicValue) -> Result<serde_json::Value> {
    decode_dynamic(&value.msgpack, &value.json)
}

fn decode_dynamic(msgpack: &[u8], json: &[u8]) -> Result<serde_json::Value> {
    if !msgpack.is_empty() {
        return rmp_serde::decode::from_slice(msgpack)
            .map_err(|e| BridgeError::Provider(format!("msgpack decode: {e}")));
    }
    if !json.is_empty() {
        return serde_json::from_slice(json)
            .map_err(|e| BridgeError::Provider(format!("json decode: {e}")));
    }
    Ok(serde_json::json!({}))
}

/// Protocol version negotiated during go-plugin handshake.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluginProtocol {
    V5,
    V6,
}

/// Connected Terraform provider process + gRPC client (v5 or v6).
pub enum TfPluginClient {
    V5 {
        provider_name: String,
        client: ProviderClientV5<Channel>,
        child: Arc<Mutex<Option<Child>>>,
    },
    V6 {
        provider_name: String,
        client: ProviderClientV6<Channel>,
        child: Arc<Mutex<Option<Child>>>,
    },
}

impl TfPluginClient {
    pub fn protocol(&self) -> PluginProtocol {
        match self {
            Self::V5 { .. } => PluginProtocol::V5,
            Self::V6 { .. } => PluginProtocol::V6,
        }
    }

    pub fn provider_name(&self) -> &str {
        match self {
            Self::V5 { provider_name, .. } | Self::V6 { provider_name, .. } => provider_name,
        }
    }

    pub async fn connect(
        binary: &Path,
        provider_name: &str,
        config: &serde_json::Value,
    ) -> Result<Self> {
        preflight_security_check(binary)?;
        let (child, hs) = launch_and_handshake(binary).await?;
        let child = Arc::new(Mutex::new(Some(child)));

        let channel = Channel::from_shared(hs.endpoint.clone())
            .map_err(|e| BridgeError::Connection(e.to_string()))?
            .connect()
            .await
            .map_err(|e| BridgeError::Connection(format!("{}: {e}", hs.endpoint)))?;

        if hs.plugin_protocol >= 6 {
            Self::connect_v6(provider_name, config, channel, child, &hs).await
        } else {
            Self::connect_v5(provider_name, config, channel, child, &hs).await
        }
    }

    async fn connect_v5(
        provider_name: &str,
        config: &serde_json::Value,
        channel: Channel,
        child: Arc<Mutex<Option<Child>>>,
        hs: &HandshakeResult,
    ) -> Result<Self> {
        if hs.plugin_protocol != 5 {
            tracing::warn!(
                "unexpected plugin protocol {} for v5 client path",
                hs.plugin_protocol
            );
        }
        let mut client = ProviderClientV5::new(channel);
        let resp = client
            .configure(tonic::Request::new(crate::tfplugin5::configure::Request {
                terraform_version: env!("CARGO_PKG_VERSION").into(),
                config: Some(json_to_dynamic_v5(config)?),
                client_capabilities: None,
            }))
            .await
            .map_err(|e| BridgeError::Provider(format!("Configure: {e}")))?;
        check_diagnostics_v5(&resp.into_inner().diagnostics)?;
        Ok(Self::V5 {
            provider_name: provider_name.to_string(),
            client,
            child,
        })
    }

    async fn connect_v6(
        provider_name: &str,
        config: &serde_json::Value,
        channel: Channel,
        child: Arc<Mutex<Option<Child>>>,
        hs: &HandshakeResult,
    ) -> Result<Self> {
        if hs.plugin_protocol < 6 {
            return Err(BridgeError::Handshake(format!(
                "provider protocol {} is not v6",
                hs.plugin_protocol
            )));
        }
        let mut client = ProviderClientV6::new(channel);
        let resp = client
            .configure_provider(tonic::Request::new(
                crate::tfplugin6::configure_provider::Request {
                    terraform_version: env!("CARGO_PKG_VERSION").into(),
                    config: Some(json_to_dynamic_v6(config)?),
                    client_capabilities: None,
                },
            ))
            .await
            .map_err(|e| BridgeError::Provider(format!("ConfigureProvider: {e}")))?;
        check_diagnostics_v6(&resp.into_inner().diagnostics)?;
        Ok(Self::V6 {
            provider_name: provider_name.to_string(),
            client,
            child,
        })
    }

    pub async fn get_schema_v5(
        &mut self,
    ) -> Result<crate::tfplugin5::get_provider_schema::Response> {
        let Self::V5 { client, .. } = self else {
            return Err(BridgeError::Provider(
                "GetSchema called on non-v5 client".into(),
            ));
        };
        let resp = client
            .get_schema(tonic::Request::new(
                crate::tfplugin5::get_provider_schema::Request {},
            ))
            .await
            .map_err(|e| BridgeError::Provider(format!("GetSchema: {e}")))?;
        Ok(resp.into_inner())
    }

    pub async fn get_schema_v6(
        &mut self,
    ) -> Result<crate::tfplugin6::get_provider_schema::Response> {
        let Self::V6 { client, .. } = self else {
            return Err(BridgeError::Provider(
                "GetProviderSchema called on non-v6 client".into(),
            ));
        };
        let resp = client
            .get_provider_schema(tonic::Request::new(
                crate::tfplugin6::get_provider_schema::Request {},
            ))
            .await
            .map_err(|e| BridgeError::Provider(format!("GetProviderSchema: {e}")))?;
        Ok(resp.into_inner())
    }

    pub async fn plan_resource_change(
        &mut self,
        type_name: &str,
        prior: &serde_json::Value,
        proposed: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        match self {
            Self::V5 { client, .. } => {
                let resp = client
                    .plan_resource_change(tonic::Request::new(
                        crate::tfplugin5::plan_resource_change::Request {
                            type_name: type_name.to_string(),
                            prior_state: Some(json_to_dynamic_v5(prior)?),
                            proposed_new_state: Some(json_to_dynamic_v5(proposed)?),
                            config: Some(json_to_dynamic_v5(proposed)?),
                            prior_private: vec![],
                            provider_meta: None,
                            client_capabilities: None,
                            prior_identity: None,
                        },
                    ))
                    .await
                    .map_err(|e| BridgeError::Provider(format!("PlanResourceChange: {e}")))?;
                let inner = resp.into_inner();
                check_diagnostics_v5(&inner.diagnostics)?;
                Ok(inner
                    .planned_state
                    .as_ref()
                    .map(dynamic_to_json_v5)
                    .transpose()?
                    .unwrap_or_else(|| proposed.clone()))
            }
            Self::V6 { client, .. } => {
                let resp = client
                    .plan_resource_change(tonic::Request::new(
                        crate::tfplugin6::plan_resource_change::Request {
                            type_name: type_name.to_string(),
                            prior_state: Some(json_to_dynamic_v6(prior)?),
                            proposed_new_state: Some(json_to_dynamic_v6(proposed)?),
                            config: Some(json_to_dynamic_v6(proposed)?),
                            prior_private: vec![],
                            provider_meta: None,
                            client_capabilities: None,
                            prior_identity: None,
                            planned_private: vec![],
                        },
                    ))
                    .await
                    .map_err(|e| BridgeError::Provider(format!("PlanResourceChange: {e}")))?;
                let inner = resp.into_inner();
                check_diagnostics_v6(&inner.diagnostics)?;
                Ok(inner
                    .planned_state
                    .as_ref()
                    .map(dynamic_to_json_v6)
                    .transpose()?
                    .unwrap_or_else(|| proposed.clone()))
            }
        }
    }

    pub async fn apply_resource_change(
        &mut self,
        type_name: &str,
        prior: &serde_json::Value,
        planned: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        match self {
            Self::V5 { client, .. } => {
                let resp = client
                    .apply_resource_change(tonic::Request::new(
                        crate::tfplugin5::apply_resource_change::Request {
                            type_name: type_name.to_string(),
                            prior_state: Some(json_to_dynamic_v5(prior)?),
                            planned_state: Some(json_to_dynamic_v5(planned)?),
                            config: Some(json_to_dynamic_v5(planned)?),
                            planned_private: vec![],
                            provider_meta: None,
                            planned_identity: None,
                        },
                    ))
                    .await
                    .map_err(|e| BridgeError::Provider(format!("ApplyResourceChange: {e}")))?;
                let inner = resp.into_inner();
                check_diagnostics_v5(&inner.diagnostics)?;
                Ok(inner
                    .new_state
                    .as_ref()
                    .map(dynamic_to_json_v5)
                    .transpose()?
                    .unwrap_or_else(|| planned.clone()))
            }
            Self::V6 { client, .. } => {
                let resp = client
                    .apply_resource_change(tonic::Request::new(
                        crate::tfplugin6::apply_resource_change::Request {
                            type_name: type_name.to_string(),
                            prior_state: Some(json_to_dynamic_v6(prior)?),
                            planned_state: Some(json_to_dynamic_v6(planned)?),
                            config: Some(json_to_dynamic_v6(planned)?),
                            planned_private: vec![],
                            provider_meta: None,
                            planned_identity: None,
                        },
                    ))
                    .await
                    .map_err(|e| BridgeError::Provider(format!("ApplyResourceChange: {e}")))?;
                let inner = resp.into_inner();
                check_diagnostics_v6(&inner.diagnostics)?;
                Ok(inner
                    .new_state
                    .as_ref()
                    .map(dynamic_to_json_v6)
                    .transpose()?
                    .unwrap_or_else(|| planned.clone()))
            }
        }
    }

    pub async fn stop(&mut self) -> Result<()> {
        match self {
            Self::V5 { client, child, .. } => {
                let _ = client
                    .stop(tonic::Request::new(crate::tfplugin5::stop::Request {}))
                    .await;
                if let Some(mut c) = child.lock().await.take() {
                    let _ = c.kill().await;
                }
            }
            Self::V6 { client, child, .. } => {
                let _ = client
                    .stop_provider(tonic::Request::new(
                        crate::tfplugin6::stop_provider::Request {},
                    ))
                    .await;
                if let Some(mut c) = child.lock().await.take() {
                    let _ = c.kill().await;
                }
            }
        }
        Ok(())
    }
}

fn check_diagnostics_v5(diags: &[crate::tfplugin5::Diagnostic]) -> Result<()> {
    for d in diags {
        if d.severity == 1 {
            return Err(BridgeError::Provider(format!("{}: {}", d.summary, d.detail)));
        }
    }
    Ok(())
}

fn check_diagnostics_v6(diags: &[crate::tfplugin6::Diagnostic]) -> Result<()> {
    for d in diags {
        if d.severity == 1 {
            return Err(BridgeError::Provider(format!("{}: {}", d.summary, d.detail)));
        }
    }
    Ok(())
}

pub fn json_to_dynamic(value: &serde_json::Value) -> Result<crate::tfplugin6::DynamicValue> {
    json_to_dynamic_v6(value)
}

pub fn dynamic_to_json(value: &crate::tfplugin6::DynamicValue) -> Result<serde_json::Value> {
    dynamic_to_json_v6(value)
}
