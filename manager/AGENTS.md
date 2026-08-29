# AGENTS.md - stignore-manager

A Rust web application that manages and aggregates data from multiple stignore agents via HTTP/JSON API endpoints.

**Part of the stignore workspace** - uses shared types and configuration from `stignore-lib`.

## Project Overview

This is a web-based manager application that:
- Communicates with multiple stignore agents in parallel over HTTP
- Aggregates and consolidates filesystem trees across agents
- Monitors redundancy copy counts, sync conflicts, and active transfers
- Coordinates media removals with Radarr and Sonarr
- Provides a dynamic HTMX web interface with Tera templating and keyboard navigation

## Architecture

- **Web Server**: Axum-based HTTP server with static file serving and compression
- **Templating**: Tera template engine with HTML templates and modular HTMX components
- **Configuration**: TOML-based configuration with `${VAR}` env var expansion and `STIGNORE_*` environment variable overrides
- **Agent Communication**: Async parallel HTTP client (`JoinSet`) with configurable timeout
- **Shared Library**: Uses `stignore-lib` for types, config, validation, and content hashing

## Key Components

### Core Modules
- `main.rs` - Application entry point, Tera setup, graceful shutdown
- `lib.rs` - Router setup, template filters (`humansize`), `AppState`
- `config.rs` - Configuration loading and environment variable override application
- `agents.rs` - Parallel agent query coordination (`JoinSet`), tree merging, recursive sorting
- `agent_client.rs` - HTTP client for agent communication
- `auth.rs` - Header-based authentication extractor and RBAC (`Admin` / `Reader`)
- `components.rs` - HTMX component endpoints and action routes
- `integrations/` - Radarr and Sonarr integration clients with media title/season matching

### Configuration
- `config.toml` - Runtime configuration (manager port, minimum copies, timeout, auth, integrations, agents)
- Supports environment variable overrides (`STIGNORE_*`) and `${VAR}` placeholders in config files

#### Authentication & RBAC
- **Agent Communication**: Authenticates with agents using `X-API-Key` headers.
- **Reverse Proxy Header Auth**: Supports `Admin` (full read/write access) and `Reader` (read-only) roles. Mutating operations return `403 Forbidden` for Reader users.

## Web Interface & Endpoints

- **Pages**: `/` (Dashboard), `/agents` (Agents overview), `/health` / `/healthz` (Health checks)
- **HTMX Components**: `/components/itemlist.html`, `/components/dynamic-items.html`, `/components/infopanel.html`, `/components/agent-modal.html`, `/components/stignore-modal.html`, `/components/agents-table.html`, `/components/agent-status-pill.html`
- **Actions**: `/components/ignore`, `/components/unignore`, `/components/delete`, `/components/delete-details`, `/components/bulk-ignore`, `/components/bulk-unignore`, `/components/bulk-delete`, `/components/agents/toggle`, `/components/stignore/save`, `/components/stignore/restore`, `/components/stignore/validate`

## Development Commands

```bash
# Build binary
cargo build --bin stignore-manager

# Run manager with config
cargo run --bin stignore-manager manager/config.toml

# Run manager integration and unit tests
cargo test --package stignore-manager

# Format and lint
cargo fmt --check
cargo clippy --all-targets --all-features
```

## UI/UX Guidelines

### User Feedback
For all user actions (ignore, delete, save, etc.), use toast notifications for immediate feedback:
- **Success**: Green toast with checkmark icon
- **Error**: Red toast with X icon
- **Implementation**: Call `showToast(message, type)` function
- Avoid browser `alert()` popups for user action feedback.
