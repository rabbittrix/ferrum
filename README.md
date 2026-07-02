# Ferrum

**Next-generation Infrastructure as Code — faster, safer, smarter.**

Ferrum is a high-performance, memory-safe IaC tool written in Rust, designed to surpass Terraform, OpenTofu, and Pulumi. Built by **Roberto de Souza** ([rabbittrix@hotmail.com](mailto:rabbittrix@hotmail.com)).

---

## Vision

| Capability | Ferrum | Terraform | OpenTofu | Pulumi |
|---|---|---|---|---|
| Language | Rust (memory-safe) | HCL + Go | HCL + Go | Multi-language |
| State encryption | **Native AES-256-GCM** | External / manual | External / manual | Service-dependent |
| Smart refactoring | **Cloud UID tracking** | Address-based (destroy/recreate) | Address-based | Varies |
| Concurrency | **Tokio + Rayon** | Provider-limited | Provider-limited | Node.js async |
| Provider ecosystem | **gRPC bridge to TF providers** | Native | Native | Native SDKs |

> **Official binaries are the recommended distribution method.**  
> The source is open on GitHub, but signed releases from this repository are the supported install path.

---

## Architecture — Data Flow

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  .fe files  │────▶│  ferrum-parser   │────▶│ ferrum-resolver │  order: VPC → Subnet → Instance
│ (typed)     │     │  type-check      │     │  DAG graph      │
└─────────────┘     └──────────────────┘     └────────┬────────┘
                                                      │
┌─────────────┐     ┌──────────────────┐              ▼
│ ferrum.fstate│◀───│   ferrum-core    │◀──── ferrum plan / apply / refresh
│ (AES-256)   │     │  Tokio + Rayon   │      (5–10× faster refresh)
└─────────────┘     └────────┬─────────┘
                             │
                    ┌────────▼─────────┐
                    │  ferrum-gui      │  real-time graph
                    │  (Tauri)         │  🔵 creating · 🟢 ok · 🔴 error · 🔐 vault
                    └──────────────────┘
```

```text
ferrum/
├── ferrum-parser/         # Strongly typed .fe language
├── ferrum-resolver/       # Creation order (VPC → Subnet → Instance)
├── ferrum-crypto/         # AES-256-GCM (state + vault)
├── ferrum-core/           # Engine: drift, cost, lock, AI, import
├── ferrum-state/          # Encrypted ferrum.fstate
├── ferrum-telemetry/      # Anonymous install notification
├── ferrum-cli/            # init, plan, apply, import, refresh
├── ferrum-provider-bridge/# gRPC → Terraform providers (Go)
└── ferrum-gui/            # Dashboard Tauri + Next.js
```

### Killer Features

| Feature | Description |
|---|---|
| **Drift Detection** | Alerts when resources are changed manually in the cloud |
| **Cost Estimation** | Monthly cost estimate on Plan (Infracost integration) |
| **Ferrum Vault** | Secrets never in plain text — 🔐 icon on graph nodes |
| **State Locking** | Distributed lock (S3/DynamoDB or native server) for teams |
| **Ferrum AI** | Diagnosis: "Why did aws_instance.web fail?" |
| **Smart Import** | `ferrum import terraform.tfstate` → generates `ferrum.graph.json` automatically |

### Providers: plugin architecture from day one

Ferrum uses a **gRPC plugin architecture** (`ferrum-provider-bridge`). AWS, Azure, and GCP are bundled official providers — any service can be added with a Go plugin compatible with Terraform.

---

## Installation

### Prerequisites

- **Rust** 1.75+ ([rustup.rs](https://rustup.rs))
- **Node.js** 20+ (for GUI only)

### Build from source

```bash
git clone https://github.com/rabbittrix/ferrum.git
cd ferrum
cargo build --release
```

The CLI binary is at `target/release/ferrum`.

### Windows (MSI / EXE)

```powershell
cargo build --release -p ferrum-cli
# GUI installer:
cd ferrum-gui
npm install
npm run tauri:build
# Output: src-tauri/target/release/bundle/msi/ and nsis/
```

### Linux (Deb / AppImage)

```bash
cargo build --release -p ferrum-cli
cd ferrum-gui && npm install && npm run tauri:build
# Output: src-tauri/target/release/bundle/deb/ and appimage/
```

---

## Quick Start

```bash
# Initialize a new project
ferrum init

# Import existing Terraform state
ferrum import ./terraform.tfstate

# Preview changes
ferrum plan

# Apply changes
ferrum apply

# Parallel cloud refresh
ferrum refresh
```

### Encrypted State

State is stored in `ferrum.fstate` (AES-256-GCM). Use a passphrase:

```bash
ferrum init --passphrase "your-secure-passphrase"
ferrum plan --passphrase "your-secure-passphrase"
```

Or set `FERRUM_STATE_KEY` (64-char hex) for CI/CD pipelines.

---

## Ferrum Language (`.fe`)

Declarative syntax — hybrid between HCL and Rust-like structs for type safety. Parsed with **pest**, validated against provider schemas, and resolved into a **DAG** before plan/apply.

### Example

```ferrum
resource "aws_vpc" "main" {
    cidr_block: "10.0.0.0/16",
    enable_dns_support: true,
    tags: {
        name: "Production-VPC"
    }
}

resource "aws_subnet" "public" {
    vpc_id: aws_vpc.main.id,   // typed cross-resource reference
    cidr_block: "10.0.1.0/24"
}
```

### Syntax rules

| Feature | Supported |
|---|---|
| Attribute assignment | `key: value` or `key = value` |
| Resource headers | `resource type name` or `resource "type" "name"` |
| Values | strings, numbers, booleans, lists, nested objects |
| References | `aws_vpc.main.id` (type-checked against symbol table) |
| Comments | `// line comments` |
| Validation errors | exact **line and column** (e.g. missing required fields) |

`ferrum plan` reads all `*.fe` files in the project directory, validates them, diffs against encrypted `ferrum.fstate`, prints a **color-coded plan**, and writes `ferrum.graph.json` for the Dashboard graph view.

See [`infra.fe`](infra.fe) for a full multi-resource example.

---

## CLI Reference

| Command | Description |
|---|---|
| `ferrum init [path]` | Initialize project with encrypted state |
| `ferrum plan` | Show execution plan |
| `ferrum apply` | Apply planned changes |
| `ferrum import <tfstate>` | Import Terraform JSON state |
| `ferrum refresh` | Parallel cloud-state refresh |
| `ferrum version` | Show version info |

### Global Flags

| Flag | Description |
|---|---|
| `--no-telemetry` | Disable anonymous first-run install notification |
| `FERRUM_TELEMETRY_DISABLED=1` | Environment variable opt-out |

---

## GUI Dashboard

Cyber-Industrial Dark theme — deep space blues, neon cyan accents, Rust orange actions.

```powershell
# From the project root:
npm run tauri:dev

# Or directly:
cd ferrum-gui
npm install
npm run tauri:dev
```

Features:

- Real-time infrastructure graph (nodes & edges)
- One-click Plan and Apply
- State history visualizer
- Secure vault manager (encrypted secrets)

---

## Provider Bridge

Ferrum communicates with Terraform-compatible Go providers via gRPC (`ferrum-provider-bridge/proto/provider.proto`):

```Text
Ferrum Core (Rust) ──gRPC──▶ Provider Bridge ──▶ Terraform Provider (Go)
```

Start a provider bridge server, then configure `ferrum.toml`:

```toml
[provider.aws]
bridge_endpoint = "http://127.0.0.1:50051"
```

---

## Telemetry

On first run, Ferrum sends a one-time anonymous HTTPS POST with OS type and version (for installation tracking). **No personal data is collected.**

Opt out:

```bash
ferrum --no-telemetry init
# or
export FERRUM_TELEMETRY_DISABLED=1
```

---

## License

Business Source License 1.1 (BUSL-1.1) — see [LICENSE](LICENSE).

---

## Author

**Roberto de Souza**  
Email: [rabbittrix@hotmail.com](mailto:rabbittrix@hotmail.com)  
GitHub: [github.com/rabbittrix/ferrum](https://github.com/rabbittrix/ferrum)

---

<p align="center">
  <strong>Ferrum</strong> — Forging infrastructure at the speed of Rust.
</p>
