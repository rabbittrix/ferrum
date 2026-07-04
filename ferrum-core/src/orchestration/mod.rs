pub mod k8s;

pub use k8s::{deploy_pod_and_service, specs_from_resource, OrchestrationResult, PodSpec, ServiceSpec};
