//! Project scaffolding templates for `ferrum init --template`.

use std::path::Path;

use anyhow::{bail, Context, Result};

pub const TEMPLATE_NAMES: &[&str] = &["docker-local", "aws-web-app", "azure-k8s-cluster"];

pub fn apply_template(dir: &Path, template: &str) -> Result<()> {
    if !TEMPLATE_NAMES.contains(&template) {
        bail!(
            "unknown template '{template}'. Available: {}",
            TEMPLATE_NAMES.join(", ")
        );
    }

    let (main_fe, ferrum_json) = match template {
        "docker-local" => (DOCKER_LOCAL_FE, DOCKER_LOCAL_JSON),
        "aws-web-app" => (AWS_WEB_APP_FE, AWS_WEB_APP_JSON),
        "azure-k8s-cluster" => (AZURE_K8S_FE, AZURE_K8S_JSON),
        _ => unreachable!(),
    };

    std::fs::write(dir.join("main.fe"), main_fe).context("write main.fe")?;
    std::fs::write(dir.join("ferrum.json"), ferrum_json).context("write ferrum.json")?;
    Ok(())
}

const DOCKER_LOCAL_FE: &str = r#"// Ferrum template: docker-local
// Author: Roberto de Souza

provider docker {
  host = "unix:///var/run/docker.sock"
}

resource docker_network app {
  name = "ferrum-app"
}

resource docker_container web {
  name  = "ferrum-web"
  image = "nginx:alpine"
  ports = ["8080:80"]
}
"#;

const DOCKER_LOCAL_JSON: &str = r#"{
  "project": { "name": "docker-local", "template": "docker-local" },
  "state": { "file": "ferrum.fstate", "encrypted": true },
  "orchestration": { "docker": true },
  "lock": { "backend": "file" },
  "telemetry": { "disabled": false }
}
"#;

const AWS_WEB_APP_FE: &str = r#"// Ferrum template: aws-web-app
// Author: Roberto de Souza

provider aws {
  region = "us-east-1"
}

resource aws_vpc main {
  cidr_block = "10.0.0.0/16"
}

resource aws_subnet public {
  vpc_id     = aws_vpc.main.id
  cidr_block = "10.0.1.0/24"
}

resource aws_security_group web {
  name   = "ferrum-web-sg"
  vpc_id = aws_vpc.main.id
}

resource aws_instance web {
  ami           = "ami-0c55b159cbfafe1f0"
  instance_type = "t3.micro"
  subnet_id     = aws_subnet.public.id
}

// High-level load balancer — Ferrum expands SG, LB, target group, listener
load_balancer main {
  target = aws_instance.web
  vpc    = aws_vpc.main
  port   = 80
}
"#;

const AWS_WEB_APP_JSON: &str = r#"{
  "project": { "name": "aws-web-app", "template": "aws-web-app" },
  "state": { "file": "ferrum.fstate", "encrypted": true },
  "providers": { "aws": { "region": "us-east-1" } },
  "lock": { "backend": "file" },
  "telemetry": { "disabled": false }
}
"#;

const AZURE_K8S_FE: &str = r#"// Ferrum template: azure-k8s-cluster
// Author: Roberto de Souza

provider azurerm {
  features = {}
}

resource azurerm_resource_group cluster {
  name     = "ferrum-k8s-rg"
  location = "East US"
}

resource k8s_deployment web {
  namespace = "default"
  image     = "nginx:alpine"
  port      = 80
}
"#;

const AZURE_K8S_JSON: &str = r#"{
  "project": { "name": "azure-k8s-cluster", "template": "azure-k8s-cluster" },
  "state": { "file": "ferrum.fstate", "encrypted": true },
  "orchestration": { "docker": false, "rancher_url": null },
  "lock": { "backend": "file" },
  "telemetry": { "disabled": false }
}
"#;
