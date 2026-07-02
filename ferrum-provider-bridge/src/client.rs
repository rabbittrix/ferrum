use tonic::transport::Channel;

use crate::error::{BridgeError, Result};
use crate::proto::provider_bridge_client::ProviderBridgeClient as GrpcClient;
use crate::proto::{ProviderInfoRequest, ReadResourceRequest};

/// gRPC client for the Ferrum Provider Bridge.
pub struct ProviderBridgeClient {
    inner: GrpcClient<Channel>,
}

impl ProviderBridgeClient {
    pub async fn connect(endpoint: impl Into<String>) -> Result<Self> {
        let endpoint = endpoint.into();
        let channel = Channel::from_shared(endpoint.clone())
            .map_err(|e| BridgeError::Connection(e.to_string()))?
            .connect()
            .await
            .map_err(|e| BridgeError::Connection(format!("{}: {}", endpoint, e)))?;

        Ok(Self {
            inner: GrpcClient::new(channel),
        })
    }

    pub async fn get_provider_info(&mut self, provider_address: &str) -> Result<crate::proto::ProviderInfoResponse> {
        let request = tonic::Request::new(ProviderInfoRequest {
            provider_address: provider_address.to_string(),
        });
        let response = self.inner.get_provider_info(request).await?;
        Ok(response.into_inner())
    }

    pub async fn read_resource(
        &mut self,
        resource_type: &str,
        cloud_uid: &str,
        current_state_json: &str,
    ) -> Result<crate::proto::ReadResourceResponse> {
        let request = tonic::Request::new(ReadResourceRequest {
            resource_type: resource_type.to_string(),
            cloud_uid: cloud_uid.to_string(),
            current_state_json: current_state_json.to_string(),
        });
        let response = self.inner.read_resource(request).await?;
        Ok(response.into_inner())
    }
}
