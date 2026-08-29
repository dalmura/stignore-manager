# AGENTS.md - stignore-agent

## Project Overview
This is a Rust-based HTTP API agent that provides a JSON API for managing filesystem items and `.stignore` files. The agent serves as an interface between a manager system and the local filesystem, allowing categorized browsing, ignore file management, Syncthing state inspection, and file deletion.

**Part of the stignore workspace** - uses shared types and configuration from `stignore-lib`.

## Architecture
- **Language**: Rust (edition 2021)
- **Web Framework**: Axum 0.8.4
- **Runtime**: Tokio (async)
- **Config Format**: TOML (supports `${VAR}` environment variable interpolation)
- **Logging**: tracing + tracing-subscriber
- **Shared Library**: Uses `stignore-lib` for types and config

## Key Components

### Core Modules
- `main.rs` - Application entry point, route configuration, `X-API-Key` auth middleware, graceful shutdown
- `filesystem.rs` - Filesystem traversal, Syncthing metadata inspection (conflict detection `.sync-conflict-*`, active sync files `.syncthing.*.tmp`, `.stversions` directory size calculation, `.stfolder` marker checks), file deletions
- `tasks.rs` - HTTP endpoint handlers

### Shared Dependencies
- Configuration loading via `stignore-lib::load_agent_config`
- Data structures from `stignore-lib::*` (`ItemGroup`, request/response types)
- Error handling from `stignore-lib::ConfigError`

### Configuration
Configured via TOML files specifying:
- Agent settings (`port`, `name`, `base_path`, `api_key`)
- Categories (`id`, `name`, `relative_path`)

#### Authentication
All API endpoints (except the help page at `/`) require authentication via the `X-API-Key` header. The API key must match the `api_key` value configured in the agent's TOML configuration file (UUID format recommended).

## API Endpoints

### Base Routes
- `GET /` - Help page with documentation link (unauthenticated)
- `GET /api/v1/categories` - List all configured categories
- `GET /api/v1/categories/{id}` - Get category details and items

### Item Management
- `POST /api/v1/items` - Get item info and children via JSON payload with hierarchical path (`item_path: Vec<String>`)
- `POST /api/v1/delete` - Delete a file or directory from disk

### Ignore File Management
- `POST /api/v1/ignore` - Add item to `.stignore` file
- `POST /api/v1/unignore` - Remove item from `.stignore` file
- `POST /api/v1/ignore-status` - Check if item is ignored
- `POST /api/v1/ignore-status-bulk` - Check ignore status for multiple items in batch
- `POST /api/v1/stignore/get` - Get raw `.stignore` contents, content hash, and backup list
- `POST /api/v1/stignore/set` - Update `.stignore` contents with optimistic locking (`expected_hash`) and automatic backup creation
- `POST /api/v1/stignore/restore` - Restore `.stignore` from an existing backup file

## Development Commands

```bash
# Build binary
cargo build --bin stignore-agent

# Run agent with config
cargo run --bin stignore-agent agent/config.toml

# Run test suite
cargo test --package stignore-agent

# Format and lint
cargo fmt --check
cargo clippy --all-targets --all-features
```

## Security Notes
- API key authentication required for all endpoints (except help page)
- API keys are sent via `X-API-Key` header
- Graceful shutdown handles `SIGINT` (Ctrl+C) and `SIGTERM` signals
