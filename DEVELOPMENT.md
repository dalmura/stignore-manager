# Development Guide

This repository is a Rust workspace consisting of three primary crates.

## Workspace Crates

### `lib/` (Shared Library)
Shared data types, API request/response models, and utilities used by both agent and manager.
- `ItemGroup` data structure for hierarchical filesystem trees.
- API payload definitions (ignore, unignore, categories, items).
- Common configuration models and error types.

### `agent/` (`stignore-agent`)
Lightweight HTTP API service running on storage nodes with access to Syncthing directories.
- Traverses local filesystem hierarchy for configured categories.
- Reads and updates local `.stignore` files.
- Endpoints: `/api/v1/categories`, `/api/v1/items`, `/api/v1/ignore`, `/api/v1/unignore`.

### `manager/` (`stignore-manager`)
Centralized aggregation web server and dashboard.
- `manager/src/`: Axum server, agent proxy client, aggregation logic, header auth/RBAC.
- `manager/src/integrations/`: Radarr and Sonarr API integration clients.
- `manager/html/`: Tera HTML templates and HTMX dynamic components.
- `manager/assets/`: Static CSS and JavaScript (keyboard navigation, modals, search).

## Directory Structure

```text
.
├── Cargo.toml            # Workspace definition and shared dependencies
├── agent/                # Agent service binary crate
├── lib/                  # Shared library crate
├── manager/              # Manager web dashboard crate
│   ├── assets/           # Static CSS/JS
│   ├── html/             # Tera templates & components
│   └── src/              # Backend routes & integrations
└── scripts/              # Helper scripts for local test fixtures and mock data
```

## Common Commands

```bash
# Build all workspace crates
cargo build

# Build specific binaries
cargo build --bin stignore-agent
cargo build --bin stignore-manager

# Run test suite
cargo test

# Format and lint
cargo fmt --check
cargo clippy --all-targets --all-features
```

## Running Locally

```bash
# Run an agent
cargo run --bin stignore-agent agent/config.toml

# Run the manager
cargo run --bin stignore-manager manager/config.toml
```

Local test environments and fake data generators are available under `scripts/`.
