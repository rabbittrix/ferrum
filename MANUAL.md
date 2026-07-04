# Ferrum Manual

**Author:** Roberto de Souza (`rabbittrix@hotmail.com`)  
**Version:** 0.1.0

Ferrum is a next-generation Infrastructure as Code (IaC) tool written in Rust. It uses `.fe` configuration files, encrypted state (`ferrum.fstate`), and optional Terraform provider plugins for real cloud APIs.

---

## Quick Start

### Four-step first-run verification

Use the **GUI Dashboard** (recommended) or the CLI:

| Step | GUI | CLI |
|------|-----|-----|
| 1. Doctor | **Doctor** panel → run checks; use **Fix It** / **Help** on warnings | `ferrum doctor` |
| 2. Init | **Terminal** → `ferrum init --template docker-local` | `ferrum init --template docker-local` |
| 3. Graph | **Graph** panel — preview dependencies | `ferrum plan` |
| 4. Apply | **Apply** (header) or **Smoke Test** for Docker hello-world | `ferrum apply` |

```bash
# CLI quick start
ferrum doctor
ferrum init --template aws-web-app
ferrum plan
ferrum apply
ferrum destroy
```

---

## Installation

### Windows

```powershell
cargo install --path ferrum-cli --force
ferrum doctor
```

Ensure `%USERPROFILE%\.cargo\bin` is on PATH (Rust installer usually adds this).

### Linux

```bash
chmod +x scripts/install-linux.sh
./scripts/install-linux.sh          # /usr/local/bin
./scripts/install-linux.sh --user   # ~/.local/bin
```

**GUI build prerequisites (Linux):**

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev pkg-config
```

---

## Building for Linux

Build the desktop app on a **Linux machine**:

```bash
cd ferrum-gui && npm install && npm run tauri:build
```

Bundles: `src-tauri/target/release/bundle/deb/` and `.../appimage/`.

---

## CLI Commands

| Command | Description |
|---------|-------------|
| `ferrum init [path]` | Create encrypted state + `ferrum.json` |
| `ferrum init --template <type>` | Scaffold from template (see below) |
| `ferrum doctor` | System health checks |
| `ferrum version` | Version, build date, OS/arch |
| `ferrum plan` | Show execution plan + cost estimate |
| `ferrum apply` | Apply planned changes |
| `ferrum apply -y` | Apply without confirmation |
| `ferrum destroy` | Remove all resources from state |
| `ferrum refresh` | Refresh cloud state |
| `ferrum import <tfstate>` | Import Terraform state |
| `ferrum provider install aws` | Install Terraform provider |
| `ferrum provider list` | List installed providers |
| `ferrum test-drive` | Hidden: Docker smoke test (nginx hello-world) |
| `ferrum test-drive --cleanup` | Remove smoke test project and container |

### Global flags

| Flag | Environment variable | Description |
|------|---------------------|-------------|
| `--no-telemetry` | `FERRUM_TELEMETRY_DISABLED=1` | Disable first-run install notification |

### Cross-shell compatibility

All interactive prompts use `stderr` + stdin and accept `y` / `yes` (case-insensitive). Works in **PowerShell**, **Bash**, and **CMD**.

---

## Templates

```bash
ferrum init --template docker-local
ferrum init --template aws-web-app
ferrum init --template azure-k8s-cluster
```

| Template | Description |
|----------|-------------|
| `docker-local` | Local Docker network + nginx container |
| `aws-web-app` | VPC, subnet, EC2, load balancer sugar |
| `azure-k8s-cluster` | Azure resource group + K8s pod deployment |

Each template generates `main.fe` and `ferrum.json`.

---

## Environment Variables

### AWS

```bash
# Linux / macOS
export AWS_ACCESS_KEY_ID="your-key"
export AWS_SECRET_ACCESS_KEY="your-secret"
export AWS_DEFAULT_REGION="us-east-1"

# Windows PowerShell
$env:AWS_ACCESS_KEY_ID = "your-key"
$env:AWS_SECRET_ACCESS_KEY = "your-secret"
$env:AWS_DEFAULT_REGION = "us-east-1"

# Windows CMD
set AWS_ACCESS_KEY_ID=your-key
set AWS_SECRET_ACCESS_KEY=your-secret
set AWS_DEFAULT_REGION=us-east-1
```

### Azure

```bash
export ARM_CLIENT_ID="..."
export ARM_CLIENT_SECRET="..."
export ARM_SUBSCRIPTION_ID="..."
export ARM_TENANT_ID="..."
```

### GCP

```bash
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/service-account.json"
```

### Ferrum

| Variable | Description |
|----------|-------------|
| `FERRUM_STATE_KEY` | Hex encryption key for CI/CD |
| `FERRUM_TELEMETRY_DISABLED` | Opt out of install notification |
| `RANCHER_URL` | Rancher server for K8s orchestration |
| `KUBECONFIG` | Path to kubeconfig for direct pod deploy |

---

## Configuration (`ferrum.json`)

```json
{
  "project": { "name": "my-app", "template": "aws-web-app" },
  "state": { "file": "ferrum.fstate", "encrypted": true },
  "lock": { "backend": "file", "remote_endpoint": null },
  "orchestration": { "docker": true, "rancher_url": null },
  "telemetry": { "disabled": false }
}
```

### State locking

| Backend | Description |
|---------|-------------|
| `file` | Local `ferrum.fstate.lock` (default) |
| `remote` | HTTP lock server at `remote_endpoint` |
| `memory` | Dev-only, no cross-process lock |

Locks are acquired automatically during `plan`, `apply`, `refresh`, and `destroy`.

---

## `.fe` Language

### Provider block

```fe
provider aws {
  region = "us-east-1"
}
```

### Resource block

```fe
resource aws_vpc main {
  cidr_block = "10.0.0.0/16"
}

resource aws_subnet public {
  vpc_id     = aws_vpc.main.id
  cidr_block = "10.0.1.0/24"
}
```

### Load balancer sugar (Ferrum-native)

Instead of configuring ALB, security groups, target groups, and listeners manually:

```fe
load_balancer main {
  target = aws_instance.web
  vpc    = aws_vpc.main
  port   = 80
}
```

Ferrum expands this into `aws_security_group`, `aws_lb`, `aws_lb_target_group`, and `aws_lb_listener`.

### Kubernetes pod orchestration

```fe
resource k8s_deployment web {
  namespace = "default"
  image     = "nginx:alpine"
  port      = 80
}
```

Ferrum deploys Pod + Service directly via the Kubernetes API (no kubectl/helm required).

---

## Getting Started Guides

### AWS

1. `ferrum doctor` — verify credentials
2. `ferrum provider install aws`
3. `ferrum init --template aws-web-app`
4. `ferrum plan && ferrum apply -y`

### Azure

1. Set `ARM_*` environment variables
2. `ferrum provider install azurerm`
3. `ferrum init --template azure-k8s-cluster`
4. `ferrum plan && ferrum apply -y`

### Docker (local)

1. Install Docker Desktop or Rancher Desktop
2. `ferrum doctor` — confirms Docker socket/pipe
3. `ferrum init --template docker-local`
4. `ferrum plan && ferrum apply -y`

On first `ferrum init`, Ferrum auto-detects Docker/Rancher and writes `orchestration` settings to `ferrum.json`.

---

## GUI Dashboard

Launch from `ferrum-gui/`:

```bash
cd ferrum-gui && npm run tauri:dev
```

Production build:

```bash
npm run tauri:build
```

### Integrated Terminal vs. external shell

| | Integrated Terminal (GUI) | PowerShell / Bash |
|--|---------------------------|-------------------|
| **Use when** | First-run verification, quick `ferrum` commands from the dashboard | Scripting, CI, full shell features |
| **Binary** | Auto-resolves `ferrum.exe` / `ferrum` (sibling, cargo bin, PATH) | Your installed `ferrum` on PATH |
| **Output** | Real-time ANSI-colored stream via Tauri events | Native terminal |
| **Examples** | `ferrum doctor`, `ferrum init --template docker-local` | Same commands |

Set `FERRUM_BIN` to override the CLI path used by the integrated terminal.

### Panels

- **Doctor** — interactive health checks with **Fix It** and **Help** on warnings
- **Smoke Test** — one-click Docker hello-world (`ferrum test-drive`); **Auto-Cleanup** removes `.ferrum-smoke-test`
- **Terminal** — xterm.js shell wired to real `ferrum-cli` subprocesses
- **Graph** — dependency visualization; **?** for help; apply colors (yellow / green / red)
- **Vault** — encrypted secrets in state

If Docker is not running, Smoke Test shows: *Install Docker to run a test.*

---

## Telemetry

On the **first successful** `ferrum doctor`, `ferrum init`, or **successful smoke test**, Ferrum sends a one-time anonymous HTTPS notification to the author (`rabbittrix@hotmail.com`) with:

- OS family (Windows / Linux / macOS) and architecture
- Ferrum version and installed providers
- Whether the **smoke test** succeeded (`smoke_test_success: true/false`)

No personal data is collected.

Opt out:

```bash
ferrum doctor --no-telemetry
# or
export FERRUM_TELEMETRY_DISABLED=1
```

---

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `ferrum` not found | Add cargo bin dir to PATH; run `ferrum doctor` |
| Lock held | Wait for other apply to finish or delete `ferrum.fstate.lock` |
| Provider errors | `ferrum provider install aws` + verify credentials |
| Docker not detected | Start Docker Desktop; on Windows use named pipe; use **Smoke Test** panel alert |
| Linux GUI won't build | Install webkit2gtk; run `ferrum doctor` for **linux_gui_deps** |
| Integrated terminal empty | Run the Ferrum desktop app (not `next dev` alone) |

---

## Project Layout

```text
my-project/
├── main.fe           # Infrastructure definition
├── ferrum.json       # Project config
├── ferrum.fstate     # Encrypted state (AES-256-GCM)
├── ferrum.graph.json # Dependency graph for GUI
└── .ferrum_key       # Local encryption key (dev)
```

---

© Roberto de Souza — Ferrum IaC
