# Development Guide

This repository is structured as a **Rust workspace** consisting of three crates:

```text
.
├── lib/                  # Shared library crate (stignore-lib)
├── agent/                # Lightweight storage node agent (stignore-agent)
├── manager/              # Centralized web dashboard & aggregator (stignore-manager)
├── scripts/              # Local development & mock data generation scripts
└── .github/workflows/    # CI/CD workflows for testing and container releases
```

---

## Workspace Crates

### 1. `lib/` (`stignore-lib`)
Shared library containing core domain models, configuration parsers, and API protocol definitions.
- **`lib/src/types.rs`**:
  - `ItemGroup`: Tree data structure with additive merging (`+` operator), copy count calculation, and Syncthing metadata flags (`has_conflicts`, `conflict_count`, `is_syncing`, `stversions_size_kb`, `stfolder_present`).
  - `SortOrder`: Multi-column sorting supporting name and size ordering.
  - `.stignore` Validation Engine: Syntax parser detecting invalid include directives or dangerous root wildcards (`validate_stignore_content`).
  - Optimistic locking hash generation (`compute_content_hash`).
  - Agent and Manager request and response types.
- **`lib/src/config.rs`**:
  - Configuration structs for Agent, Manager, Auth/RBAC, and Media Integrations (Radarr/Sonarr).
  - `expand_env_vars`: Dynamic interpolation for `${VAR}` and `$VAR` placeholders in TOML files.
  - `resolve_config_path` & `apply_manager_env_overrides`: Environment variable override engine.

---

### 2. `agent/` (`stignore-agent`)
Lightweight HTTP API service deployed directly on storage nodes hosting Syncthing directories.
- **`agent/src/main.rs`**: Axum application setup, route configuration, `X-API-Key` auth middleware, and graceful shutdown signal listener.
- **`agent/src/filesystem.rs`**: Local directory traversal, metadata extraction (conflict files `.sync-conflict-*`, temporary sync files `.syncthing.*.tmp`, `.stversions` directory size, `.stfolder` marker detection), and disk deletion.
- **`agent/src/tasks.rs`**: HTTP route handlers for categories, hierarchical item traversal, `.stignore` rule additions/removals, bulk ignore queries, and raw `.stignore` get/set/restore operations with automated backups.

---

### 3. `manager/` (`stignore-manager`)
Centralized aggregation web server and dynamic HTMX dashboard.
- **`manager/src/main.rs`**: Application entry point, tracing initialization, Tera template engine loading, and HTTP listener.
- **`manager/src/lib.rs`**: Router definition, template filters (`humansize`), and shared `AppState`.
- **`manager/src/agent_client.rs`**: Async HTTP client for communicating with storage agents with configurable timeouts.
- **`manager/src/agents.rs`**: High-concurrency parallel agent query executor (`JoinSet`), tree merging, and recursive sorting.
- **`manager/src/auth.rs`**: Header-based authentication extractor and RBAC permission checks (`Admin` vs `Reader`).
- **`manager/src/components.rs`**: HTMX component handlers (dynamic item table, item info panel, modals, bulk operations, stignore editor).
- **`manager/src/integrations/`**:
  - `mod.rs`: Integration manager dispatching media coordination actions.
  - `radarr.rs`: Radarr API client (movie matching, zero-copy deletion cleanup, import exclusions).
  - `sonarr.rs`: Sonarr API client (series/season matching, season unmonitoring, import list exclusions).
  - `matcher.rs`: Fuzzy media title, year, and season number parsing.
- **`manager/html/`**: Tera HTML templates and modular HTMX component snippets.
- **`manager/assets/`**: Static CSS and JavaScript (keyboard navigation, toast notifications, search, modals).

---

## Development Commands

```bash
# Build all workspace crates
cargo build

# Build specific binaries
cargo build --bin stignore-agent
cargo build --bin stignore-manager

# Run the complete test suite
cargo test

# Run tests for a specific crate or integration test file
cargo test --package stignore-lib
cargo test --package stignore-agent
cargo test --package stignore-manager
cargo test --test integration_htmx_endpoints

# Linting and formatting
cargo fmt --check
cargo clippy --all-targets --all-features
```

---

## Local Development Scripts

The `scripts/` directory contains helper scripts to quickly stand up a multi-agent test environment:

```bash
# 1. Generate test agent configurations and mock directory trees
./scripts/create_agents.sh
./scripts/create_fake_data.sh

# 2. Launch 3 background test agents (listening on ports 9001, 9002, 9003)
./scripts/run_agents.sh

# 3. Launch the manager pointing to the test agents (port 8000)
./scripts/run_manager.sh

# To reset and start clean:
./scripts/fresh_start.sh
```

---

## Key Architectural Patterns

### 1. High-Performance Parallel Queries
When browsing directories or inspecting status across multiple storage nodes, the manager issues concurrent async HTTP requests to all online agents using `tokio::task::JoinSet`. Results are merged into a single consolidated `ItemGroup` tree.

### 2. Optimistic Concurrency for `.stignore` Editing
Direct edits to `.stignore` files pass an `expected_hash`. If another process or user modified the file in the interim, the agent rejects the update with a conflict error rather than overwriting changes. Automated timestamped backups are generated prior to any write.

### 3. Role-Based Access Control (RBAC)
When reverse proxy authentication is enabled, user identities and roles are extracted from request headers. The `Reader` role is strictly restricted to read-only views; any mutating operation (ignore, unignore, delete, bulk actions, or `.stignore` updates) is rejected with `403 Forbidden`.

### 4. Media Manager Coordination
When deleting media files, the manager queries configured Radarr and Sonarr instances to identify matching movie titles or series seasons. If the final copy is being removed, the manager can automatically unmonitor the content or add an import exclusion to prevent unwanted re-downloads.

---

## Release Workflow

1. Bump workspace version in root `Cargo.toml` (`[workspace.package] version = "X.Y.Z"`).
2. Validate locally (syncs `Cargo.lock`):
   ```bash
   cargo fmt --check && cargo clippy --all-targets --all-features && cargo test
   ```
3. Commit and push:
   ```bash
   git commit -am "chore: bump version to vX.Y.Z" && git push origin main
   ```
4. Tag and push release:
   ```bash
   git tag vX.Y.Z && git push --tags
   ```
5. GitHub Actions (`.github/workflows/cd.yaml`) will build multi-arch container images, publish to GHCR, and create GitHub Releases.
