//! Official Terraform provider registry metadata.

use serde::Deserialize;

#[derive(Clone, Debug)]
pub struct ProviderSpec {
    pub name: &'static str,
    pub namespace: &'static str,
    pub display_name: &'static str,
    pub default_version: &'static str,
}

pub const OFFICIAL_PROVIDERS: &[ProviderSpec] = &[
    ProviderSpec {
        name: "aws",
        namespace: "hashicorp",
        display_name: "AWS",
        default_version: "5.82.2",
    },
    ProviderSpec {
        name: "azurerm",
        namespace: "hashicorp",
        display_name: "Azure",
        default_version: "4.14.0",
    },
    ProviderSpec {
        name: "google",
        namespace: "hashicorp",
        display_name: "GCP",
        default_version: "6.14.1",
    },
];

pub fn find_provider(name: &str) -> Option<&'static ProviderSpec> {
    OFFICIAL_PROVIDERS
        .iter()
        .find(|p| p.name == name || p.display_name.eq_ignore_ascii_case(name))
}

pub fn provider_address(spec: &ProviderSpec) -> String {
    format!("registry.terraform.io/{}/{}", spec.namespace, spec.name)
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct RegistryDownloadResponse {
    pub protocols: Vec<String>,
    pub os: String,
    pub arch: String,
    pub filename: String,
    pub download_url: String,
    pub shasums_url: String,
    pub shasums_signature_url: String,
    pub shasum: String,
}

pub fn os_arch() -> (&'static str, &'static str) {
    match std::env::consts::OS {
        "windows" => ("windows", "amd64"),
        "macos" => ("darwin", "amd64"),
        "linux" => ("linux", "amd64"),
        other => ("linux", if std::env::consts::ARCH == "x86_64" { "amd64" } else { other }),
    }
}

pub fn binary_name(spec: &ProviderSpec, version: &str) -> String {
    let (os, arch) = os_arch();
    let base = format!(
        "terraform-provider-{}_{}_{}_{}",
        spec.name, version, os, arch
    );
    if os == "windows" {
        format!("{base}.exe")
    } else {
        base
    }
}
