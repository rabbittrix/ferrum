//! Direct Kubernetes / Rancher orchestration without kubectl/helm.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{CoreError, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PodSpec {
    pub name: String,
    pub namespace: String,
    pub image: String,
    pub port: u16,
    pub labels: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceSpec {
    pub name: String,
    pub namespace: String,
    pub port: u16,
    pub target_port: u16,
    pub selector: Value,
}

#[derive(Clone, Debug)]
pub struct OrchestrationResult {
    pub pod: String,
    pub service: String,
}

/// Deploy a pod + ClusterIP service via the Kubernetes API.
pub async fn deploy_pod_and_service(spec: &PodSpec, svc: &ServiceSpec) -> Result<OrchestrationResult> {
    let client = K8sClient::from_env()?;
    client.apply_pod(spec).await?;
    client.apply_service(svc).await?;
    Ok(OrchestrationResult {
        pod: format!("{}/{}", spec.namespace, spec.name),
        service: format!("{}/{}", svc.namespace, svc.name),
    })
}

struct K8sClient {
    base_url: String,
    token: String,
    http: reqwest::Client,
}

impl K8sClient {
    fn from_env() -> Result<Self> {
        let base = std::env::var("KUBERNETES_SERVICE_HOST")
            .ok()
            .map(|h| format!("https://{h}"))
            .or_else(|| std::env::var("KUBECONFIG_SERVER").ok())
            .or_else(|| detect_rancher_k8s_url())
            .unwrap_or_else(|| "https://127.0.0.1:6443".into());

        let token = std::env::var("KUBERNETES_SERVICE_ACCOUNT_TOKEN")
            .or_else(|_| std::env::var("K8S_TOKEN"))
            .or_else(|_| read_token_from_kubeconfig())
            .unwrap_or_default();

        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| CoreError::Provider(format!("k8s client: {e}")))?;

        Ok(Self {
            base_url: base,
            token,
            http,
        })
    }

    async fn apply_pod(&self, spec: &PodSpec) -> Result<()> {
        let url = format!(
            "{}/api/v1/namespaces/{}/pods",
            self.base_url.trim_end_matches('/'),
            spec.namespace
        );
        let body = json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": {
                "name": spec.name,
                "labels": spec.labels,
            },
            "spec": {
                "containers": [{
                    "name": spec.name,
                    "image": spec.image,
                    "ports": [{ "containerPort": spec.port }],
                }]
            }
        });
        self.post_or_patch(&url, &body).await
    }

    async fn apply_service(&self, spec: &ServiceSpec) -> Result<()> {
        let url = format!(
            "{}/api/v1/namespaces/{}/services",
            self.base_url.trim_end_matches('/'),
            spec.namespace
        );
        let body = json!({
            "apiVersion": "v1",
            "kind": "Service",
            "metadata": { "name": spec.name },
            "spec": {
                "selector": spec.selector,
                "ports": [{
                    "port": spec.port,
                    "targetPort": spec.target_port,
                    "protocol": "TCP",
                }],
                "type": "ClusterIP",
            }
        });
        self.post_or_patch(&url, &body).await
    }

    async fn post_or_patch(&self, url: &str, body: &Value) -> Result<()> {
        let mut req = self.http.post(url).json(body);
        if !self.token.is_empty() {
            req = req.bearer_auth(&self.token);
        }
        let resp = req
            .send()
            .await
            .map_err(|e| CoreError::Provider(format!("k8s request: {e}")))?;

        if resp.status().is_success() || resp.status().as_u16() == 409 {
            return Ok(());
        }

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        Err(CoreError::Provider(format!("k8s API {status}: {text}")))
    }
}

fn detect_rancher_k8s_url() -> Option<String> {
    let rancher = std::env::var("RANCHER_URL").ok()?;
    Some(format!("{rancher}/k8s/clusters/local"))
}

fn read_token_from_kubeconfig() -> Result<String> {
    let path = std::env::var("KUBECONFIG")
        .ok()
        .map(|p| PathBuf::from(p))
        .or_else(|| {
            dirs::home_dir().map(|h| h.join(".kube").join("config"))
        });

    let path = path.ok_or_else(|| CoreError::Provider("KUBECONFIG not set".into()))?;
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| CoreError::Provider(format!("read kubeconfig: {e}")))?;

    for line in raw.lines() {
        if line.trim_start().starts_with("token:") {
            let token = line.split(':').nth(1).unwrap_or("").trim();
            if !token.is_empty() {
                return Ok(token.to_string());
            }
        }
    }

    Err(CoreError::Provider("no token in kubeconfig".into()))
}

/// Parse `k8s_deployment` resources from attributes into orchestration specs.
pub fn specs_from_resource(
    name: &str,
    attrs: &Value,
) -> Option<(PodSpec, ServiceSpec)> {
    let image = attrs.get("image")?.as_str()?;
    let namespace = attrs
        .get("namespace")
        .and_then(|v| v.as_str())
        .unwrap_or("default");
    let port = attrs
        .get("port")
        .and_then(|v| v.as_u64())
        .unwrap_or(80) as u16;

    let labels = json!({ "app": name });
    let pod = PodSpec {
        name: name.to_string(),
        namespace: namespace.to_string(),
        image: image.to_string(),
        port,
        labels: labels.clone(),
    };
    let svc = ServiceSpec {
        name: format!("{name}-svc"),
        namespace: namespace.to_string(),
        port,
        target_port: port,
        selector: labels,
    };
    Some((pod, svc))
}
