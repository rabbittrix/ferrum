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

## Architecture

```Rust
ferrum/
├── ferrum-core/           # Engine: dependency graph, plan/apply, concurrency
├── ferrum-state/          # AES-256-GCM encrypted ferrum.fstate
├── ferrum-cli/            # CLI: init, plan, apply, import, refresh
├── ferrum-provider-bridge/# gRPC bridge to Terraform Go providers
└── ferrum-gui/            # Tauri + Next.js desktop dashboard
```

### Key Features

- **Zero-Trust State** — `ferrum.fstate` is encrypted with AES-256-GCM. Secrets never touch disk in plain text.
- **Smart Refactoring** — Resources tracked by cloud-native UIDs. Rename in code without destroy/recreate.
- **Fearless Concurrency** — Parallel cloud-state refresh via Tokio and Rayon.
- **Terraform Migration** — `ferrum import <terraform.tfstate>` converts JSON state to encrypted Ferrum format.
- **Provider Bridge** — gRPC interface to existing Terraform providers (AWS, Azure, GCP on day one).

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

```bash
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
